pub mod connection;
pub use connection::Connection;
mod route;
use route::Route;
use spru_message::{header, payload, Header, Message, Payload};

use crate::util;

use std::{any, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use smol::lock::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(usize);

#[derive(Debug)]
enum BindState {
    Unbound(std::net::SocketAddr),
    Bound(smol::net::TcpListener),
    Invalid,
}

pub struct TcpListener<P> {
    router: Router<P>,
    bind_state: BindState,
}

impl<P> TcpListener<P> {
    async fn tcp_listener(&mut self) -> std::io::Result<&smol::net::TcpListener> {
        let bind_state = std::mem::replace(&mut self.bind_state, BindState::Invalid);

        let tcp_listener = match bind_state {
            BindState::Unbound(addr) => {
                match smol::net::TcpListener::bind(&addr).await {
                    Ok(tcp_listener) => tcp_listener,
                    Err(err) => {
                        // Reset to Unbound so we do not ever leave this function invalid
                        self.bind_state = BindState::Unbound(addr);
                        return Err(err);
                    }
                }
            },
            BindState::Bound(tcp_listener) => tcp_listener,
            BindState::Invalid => unreachable!(),
        };

        self.bind_state = BindState::Bound(tcp_listener);
        let BindState::Bound(tcp_listener) = &self.bind_state else { unreachable!() };
        Ok(tcp_listener)
    }

    pub async fn bind(&mut self) -> std::io::Result<std::net::SocketAddr> {
        self.tcp_listener().await?.local_addr()
    }

    pub async fn listen(mut self) -> std::io::Result<()> 
    where 
        P: 'static 
    {
        // Force binding before destructuring
        self.tcp_listener().await?;

        let Self {
            router,
            bind_state,
        } = self;

        let BindState::Bound(tcp_listener) = bind_state 
            else { unreachable!("tcp_listener() ensures we have already bound") };
        
        let header_len = Header::byte_length();
        
        let result: std::io::Result<()> = moro::async_scope!(|scope| {
            use moro::prelude::*;

            loop {
                let (mut tcp_stream, addr) = tcp_listener.accept().await
                    .unwrap_or_cancel(scope).await;
    
                let route = route::Tcp::new(addr, tcp_stream.clone());
                let id = router.add_route(Route::Tcp(route)).await;
    
                let router = router.clone();
                let sender = router.sender();

                let _ = scope.spawn(async move {
                    let mut header_buffer = vec![0u8; header_len];
                    
                    loop {
                        use smol::io::AsyncReadExt as _;
                        
                        tcp_stream.read_exact(&mut header_buffer).await
                            .map_err(util::discard)?;
    
                        let Ok(header) = Header::try_from(&*header_buffer) else {
                            // TODO error handling
                            return Err(());
                        };
                        
                        if header.payload_size > crate::util::PAYLOAD_MAX_LEN {
                            // TODO error handling
                            return Err(());
                        }
    
                        let mut payload_buffer = vec![0u8; header.payload_size];
    
                        tcp_stream.read_exact(&mut *payload_buffer).await
                            .map_err(util::discard)?;
    
                        let message = Message::from_bytes(header, payload_buffer.into_boxed_slice());
                        sender.send(Routed { client_id: id, value: message.into() }).await
                            .map_err(util::discard)?;
                    }

                    Ok(())
                });
            }
        }).await;

        result
    }
}

#[derive(Debug, Clone)]
pub struct Routed<T> {
    pub client_id: Id,
    pub value: T,
}

impl<T> Routed<T> {
    pub fn new(client_id: Id, value: T) -> Self {
        Self {
            client_id,
            value,
        }
    }

    fn map<U, V>(self, f: impl FnOnce(T) -> Result<U, V>) -> Result<Routed<U>, V> {
        Ok(Routed {
            client_id: self.client_id,
            value: f(self.value)?,
        })
    }
}

#[derive(Debug)]
pub struct Router<P> {
    routes: Arc<RwLock<Vec<Route<P>>>>,

    send_keep_open: smol::channel::Sender<Routed<Message<Payload<P>>>>,
    recv: smol::channel::Receiver<Routed<Message<Payload<P>>>>,
}

impl<P> Clone for Router<P> {
    fn clone(&self) -> Self {
        Self { routes: self.routes.clone(), send_keep_open: self.send_keep_open.clone(), recv: self.recv.clone() }
    }
}

impl<P> Router<P> {
    pub fn new() -> Self {
        let (send, recv) = smol::channel::unbounded();
        
        Self {
            routes: Arc::new(RwLock::new(vec![])),
            send_keep_open: send,
            recv,
        }
    }

    pub fn create_tcp_listener(&self, addr: impl Into<std::net::SocketAddr>) -> TcpListener<P> {
        let router = self.clone();

        TcpListener {
            router,
            bind_state: BindState::Unbound(addr.into()),
        }
    }

    pub async fn create_local_connection(&self) -> connection::Local<P> {
        let (send, recv) = smol::channel::unbounded();

        let route = Route::Local(route::Local::new(send));
        
        let id = self.add_route(route).await;

        let send = self.sender();
        
        connection::Local::new( send, recv, id)
    }

    async fn add_route(&self, route: Route<P>) -> Id {
        let index = {
            let mut routes = self.routes.write().await;
            let index = routes.len();
            routes.push(route);

            index
        };

        Id(index)
    }

    pub async fn send<V>(&self, data: Routed<V>) -> Result<(), V>
    where
        V: any::Any + Serialize + Send,
        P: payload::Variant<V>,
    {
        let Routed {
            client_id,
            value,
        } = data;
        let routes = self.routes.read().await;
        let route = &routes[client_id.0];
        route.send(value).await
    }

    pub fn send_blocking<V>(&self, data: Routed<V>) -> Result<(), V> 
    where
        V: any::Any + Serialize + Send,
        P: payload::Variant<V>,
    {
        let Routed {
            client_id,
            value,
        } = data;
        let routes = self.routes.read_blocking();
        let route = &routes[client_id.0];
        route.send_blocking(value)
    }

    pub async fn recv<V>(&self) -> Result<Routed<V>, crate::TempError> 
    where 
        V: any::Any + DeserializeOwned + Send,
        P: payload::Variant<V>,
    {
        self.recv_untyped().await?
            .map(Message::<Payload<P>>::into_variant)
            .map_err(util::discard)
    }

    pub async fn recv_untyped(&self) -> Result<Routed<Message<Payload<P>>>, crate::TempError> {
        self.recv.recv().await
            .map_err(util::discard)
    }

    pub fn try_recv<V>(&self) -> Result<Routed<V>, crate::TempError> 
    where
        V: any::Any + DeserializeOwned + Send,
        P: payload::Variant<V>,
    {
        self.try_recv_untyped()?
            .map(Message::<Payload<P>>::into_variant)
            .map_err(util::discard)
    }

    pub fn try_recv_untyped(&self) -> Result<Routed<Message<Payload<P>>>, crate::TempError> {
        self.recv.try_recv()
            .map_err(util::discard)
    }

    fn sender(&self) -> smol::channel::Sender<Routed<Message<Payload<P>>>> {
        self.send_keep_open.clone()
    }

    pub async fn close_route(&self, id: Id) -> Result<(), crate::TempError> {
        let mut routes = self.routes.write_arc().await;
        match routes.get_mut(id.0) {
            Some(route) => route.close().await,
            None => Err(Default::default()),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use futures_lite::FutureExt;
    use spru_message::header;

    use super::*;

    #[test]
    fn local_routing() {
        let executor = smol::LocalExecutor::new();

        #[spru_message::payload_variant(0 => i32)]
        struct Pload;

        let router = Router::<Pload>::new();

        executor.spawn({
            let router = router.clone();
            async move {
                let local0 = router.create_local_connection().await;
                let local1 = router.create_local_connection().await;
                let local2 = router.create_local_connection().await;

                local2.send(12i32).await.unwrap();
                local0.send(25i32).await.unwrap();
                local0.send(50i32).await.unwrap();
                local1.send(1i32).await.unwrap();
                local2.send(24i32).await.unwrap();
                local1.send(2i32).await.unwrap();
                local0.send(25i32).await.unwrap();
                local1.send(97i32).await.unwrap();
                local2.send(64i32).await.unwrap();
            }
        }).detach();

        let map = smol::block_on(executor.run(
            async move {
                let mut map = HashMap::<_, i32>::new();

                for _ in 0..9 {
                    let Routed { client_id: id, value } = router.recv::<i32>().await
                        .unwrap();
                    *map.entry(id).or_default() += value;
                }

                map
            }
        ));

        assert_eq!(map.len(), 3);
        for v in map.into_values() {
            assert_eq!(v, 100i32);
        }
    }

    #[test]
    fn tcp_routing() {
        #[spru_message::payload_variant(0 => i32)]
        struct Pload;

        let executor = smol::LocalExecutor::new();

        let router = Router::<Pload>::new();
        let mut tcp_listener = router.create_tcp_listener((std::net::Ipv4Addr::LOCALHOST, 0));
        let local_socket = smol::block_on(tcp_listener.bind())
            .expect("socket binding should not fail");
        println!("TcpListener binding on {}", local_socket);

        executor.spawn(tcp_listener.listen()).detach();

        executor.spawn({
            async move {
                let mut tcp0 = connection::Tcp::<Pload>::new(local_socket).await.unwrap();
                println!("TcpStream 0 connected on {} -> {}", tcp0.local_addr().unwrap(), tcp0.peer_addr().unwrap());
                let mut tcp1 = connection::Tcp::<Pload>::new(local_socket).await.unwrap();
                println!("TcpStream 1 connected on {} -> {}", tcp1.local_addr().unwrap(), tcp1.peer_addr().unwrap());
                let mut tcp2 = connection::Tcp::<Pload>::new(local_socket).await.unwrap();
                println!("TcpStream 2 connected on {} -> {}", tcp2.local_addr().unwrap(), tcp2.peer_addr().unwrap());

                tcp2.send(12i32).await.unwrap();
                tcp0.send(25i32).await.unwrap();
                tcp0.send(50i32).await.unwrap();
                tcp1.send(1i32).await.unwrap();
                tcp2.send(24i32).await.unwrap();
                tcp1.send(2i32).await.unwrap();
                tcp0.send(25i32).await.unwrap();
                tcp1.send(97i32).await.unwrap();
                tcp2.send(64i32).await.unwrap();
            }
        }).detach();

        let map = smol::block_on(executor.run(
            async move {
                smol::Timer::after(std::time::Duration::from_secs(10)).await;
                Err(())
            }.race(
                async move {
                    let mut map = HashMap::<_, i32>::new();

                    for _ in 0..9 {
                        let Routed { client_id: id, value } = router.recv::<i32>().await
                            .unwrap();
                        println!("{id:?}: {}", value);
                        
                        *map.entry(id).or_default() += value;
                    }

                    Ok(map)
                }
            )
        )).expect("Timeout");

        assert_eq!(map.len(), 3);
        for v in map.into_values() {
            assert_eq!(v, 100i32);
        }
    }
}