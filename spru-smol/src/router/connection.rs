use std::any;

use crate::{Routed, router, util};

#[derive(Debug, Clone)]
pub enum Connection<P> {
    Tcp(Tcp<P>),
    Local(Local<P>),
}

impl<P> Connection<P> {
    pub async fn send(&mut self, payload: P) -> Result<(), crate::TempError>
    where 
        P: any::Any + Send + serde::Serialize,
    {
        match self {
            Connection::Tcp(t) => t.send(payload).await,
            Connection::Local(l) => l.send(payload).await,
        }
    }

    pub fn send_blocking(&mut self, payload: P) -> Result<(), crate::TempError>
    where 
        P: any::Any + Send + serde::Serialize,
    {
        match self {
            Connection::Tcp(c) => c.send_blocking(payload),
            Connection::Local(c) => c.send_blocking(payload),
        }
    }

    pub async fn recv(&mut self) -> Result<P, crate::TempError>
    where 
        P: any::Any + Send + serde::de::DeserializeOwned,
    {
        match self {
            Connection::Tcp(t) => t.recv().await,
            Connection::Local(l) => l.recv().await,
        }
    }

    pub fn try_recv(&mut self) -> Result<P, crate::TempError>
    where 
        P: any::Any + Send + serde::de::DeserializeOwned,
    {
        match self {
            Connection::Tcp(c) => c.try_recv(),
            Connection::Local(c) => c.try_recv(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tcp<P> {
    stream: smol::net::TcpStream,
    _p: std::marker::PhantomData<P>,
}

impl<P> Tcp<P> {
    pub async fn new<A: smol::net::AsyncToSocketAddrs>(addr: A) -> Result<Self, crate::TempError> {
        let stream = smol::net::TcpStream::connect(addr).await
            .map_err(util::discard)?;
        Ok(Self {
            stream,
            _p: Default::default(),
        })
    }

    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.stream.local_addr()
    }

    pub fn peer_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.stream.peer_addr()
    }

    pub async fn send(&mut self, payload: P) -> Result<(), crate::TempError>
    where
        P: serde::Serialize,
    {
        let mut buffer = vec![0u8; util::PAYLOAD_MAX_LEN];
        util::serialize_over_stream(self.stream.clone(), &mut buffer, &payload)
            .await
            .map_err(util::discard)?;
        
        Ok(())
    }

    pub fn send_blocking(&mut self, payload: P) -> Result<(), crate::TempError>
    where
        P: serde::Serialize,
    {
        smol::block_on(self.send(payload))
    }

    pub async fn recv(&mut self) -> Result<P, crate::TempError>
    where
        P: serde::de::DeserializeOwned,
    {
        let mut buffer = vec![0u8; util::PAYLOAD_MAX_LEN];
            
        let payload = util::deserialize_over_stream(self.stream.clone(), &mut buffer)
            .await
            .map_err(util::discard)?;

        Ok(payload)
    }

    pub fn try_recv(&mut self) -> Result<P, crate::TempError>
    where
        P: serde::de::DeserializeOwned,
    {
        smol::block_on(smol::future::poll_once(self.recv()))
            .unwrap_or(Err(crate::TempError))
    }
}

#[derive(Debug, Clone)]
pub struct Local<P> {
    send: smol::channel::Sender<Routed<P>>,
    recv: smol::channel::Receiver<P>,
    client_id: router::Id,
}

impl<P> Local<P> {
    pub(crate) fn new(send: smol::channel::Sender<Routed<P>>, recv: smol::channel::Receiver<P>, client_id: router::Id) -> Self {
        Self {
            send,
            recv,
            client_id,
        }
    }

    pub fn router_id(&self) -> router::Id {
        self.client_id
    }

    pub async fn send(&self, payload: P) -> Result<(), crate::TempError> 
    where 
        P: std::any::Any + Send,
    {
        let message = Routed {
            client_id: self.client_id,
            value: payload,
        };
        self.send.send(message)
            .await
            .map_err(util::discard)
    }

    pub fn send_blocking(&self, payload: P) -> Result<(), crate::TempError>
    where 
        P: std::any::Any + Send,
    {
        let message = Routed {
            client_id: self.client_id,
            value: payload,
        };
        self.send.send_blocking(message)
            .map_err(util::discard)
    }

    pub async fn recv(&self) -> Result<P, crate::TempError> 
    where 
        P: std::any::Any + Send,
    {
        self.recv.recv().await
            .map_err(util::discard)
    }

    pub fn try_recv(&self) -> Result<P, crate::TempError>
    where 
        P: std::any::Any + Send,
    {
        self.recv.try_recv()
            .map_err(util::discard)
    }
}