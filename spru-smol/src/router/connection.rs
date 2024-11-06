use std::any;

use futures_lite::AsyncReadExt;
use spru_message::{header, payload, Message, Payload};

use crate::{Routed, router, util};

#[derive(Debug, Clone)]
pub enum Connection<P> {
    Tcp(Tcp<P>),
    Local(Local<P>),
}

impl<P> Connection<P> {
    pub async fn send<V>(&mut self, payload: V) -> Result<(), crate::TempError>
    where 
        V: serde::Serialize + any::Any + Send,
        P: payload::Variant<V>,
    {
        match self {
            Connection::Tcp(t) => t.send(payload).await,
            Connection::Local(l) => l.send(payload).await,
        }
    }

    pub fn send_blocking<V>(&mut self, payload: V) -> Result<(), crate::TempError>
    where 
        V: serde::Serialize + any::Any + Send,
        P: payload::Variant<V>,
    {
        match self {
            Connection::Tcp(c) => c.send_blocking(payload),
            Connection::Local(c) => c.send_blocking(payload),
        }
    }

    pub async fn recv<V>(&mut self) -> Result<V, crate::TempError>
    where 
        V: serde::de::DeserializeOwned + any::Any + Send,
        P: payload::Variant<V>,
    {
        match self {
            Connection::Tcp(t) => t.recv().await,
            Connection::Local(l) => l.recv().await,
        }
    }

    pub fn try_recv<V>(&mut self) -> Result<V, crate::TempError>
    where 
        V: serde::de::DeserializeOwned + any::Any + Send,
        P: payload::Variant<V>,
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

    pub async fn send<V>(&mut self, payload: V) -> Result<(), crate::TempError>
    where
        P: payload::Variant<V>,
        V: serde::Serialize,
    {
        use futures_lite::AsyncWriteExt as _;
        
        let message = Message::<P>::new_serialized(&payload)
            .map_err(util::discard)?;

        let bytes = message.into_bytes();
        
        self.stream.write_all(&*bytes).await
            .map_err(util::discard)?;
        
        Ok(())
    }

    pub fn send_blocking<V>(&mut self, payload: V) -> Result<(), crate::TempError>
    where
        P: payload::Variant<V>,
        V: serde::Serialize,
    {
        smol::block_on(self.send(payload))
    }

    pub async fn recv<V>(&mut self) -> Result<V, crate::TempError>
    where
        P: payload::Variant<V>,
        V: serde::de::DeserializeOwned,
    {
        use futures_lite::AsyncReadExt as _;

        let header_len = spru_message::Header::byte_length();

        let mut header_buffer = vec![0u8; header_len];
            
        self.stream.read_exact(&mut header_buffer).await
            .map_err(util::discard)?;

        let Ok(header) = spru_message::Header::try_from(&*header_buffer) else {
            // TODO error handling
            return Err(crate::TempError);
        };
        
        if header.payload_size > crate::util::PAYLOAD_MAX_LEN {
            // TODO error handling
            return Err(crate::TempError);
        }

        let mut payload_buffer = vec![0u8; header.payload_size];

        self.stream.read_exact(&mut *payload_buffer).await
            .map_err(util::discard)?;

        let message = Message::<P>::from_bytes(header, payload_buffer.into_boxed_slice());

        let v = message.into_variant()
            .map_err(util::discard)?;

        Ok(v)
    }

    pub fn try_recv<V>(&mut self) -> Result<V, crate::TempError>
    where
        P: payload::Variant<V>,
        V: serde::de::DeserializeOwned,
    {
        smol::block_on(smol::future::poll_once(self.recv()))
            .unwrap_or(Err(crate::TempError))
    }
}

#[derive(Debug, Clone)]
pub struct Local<P> {
    send: smol::channel::Sender<Routed<Message<Payload<P>>>>,
    recv: smol::channel::Receiver<Message<payload::Raw<P>>>,
    client_id: router::Id,
}

impl<P> Local<P> {
    pub(crate) fn new(send: smol::channel::Sender<Routed<Message<Payload<P>>>>, recv: smol::channel::Receiver<Message<payload::Raw<P>>>, client_id: router::Id) -> Self {
        Self {
            send,
            recv,
            client_id,
        }
    }

    pub fn router_id(&self) -> router::Id {
        self.client_id
    }

    pub async fn send<V>(&self, payload: V) -> Result<(), crate::TempError> 
    where 
        P: payload::Variant<V>,
        V: std::any::Any + Send,
    {
        let message = Routed {
            client_id: self.client_id,
            value: Message::<P>::new_raw(payload).into(),
        };
        self.send.send(message).await
            .map_err(|e| {
                use payload::IntoVariant as _;
                let Payload::Raw(r) = e.0.value.payload else { unreachable!("Message was created as Raw locally") };
                let Ok(v) = r.into_variant() else { unreachable!("Variant was cast locally") };
                v
            })
            .map_err(util::discard)
    }

    pub fn send_blocking<V>(&self, payload: V) -> Result<(), crate::TempError>
    where 
        P: payload::Variant<V>,
        V: std::any::Any + Send,
    {
        let message = Routed {
            client_id: self.client_id,
            value: Message::<P>::new_raw(payload).into(),
        };
        self.send.send_blocking(message)
            .map_err(|e| {
                use payload::IntoVariant as _;
                let Payload::Raw(r) = e.0.value.payload else { unreachable!("Message was created as Raw locally") };
                let Ok(v) = r.into_variant() else { unreachable!("Variant was cast locally") };
                v
            })
            .map_err(util::discard)
    }

    pub async fn recv<V>(&self) -> Result<V, crate::TempError> 
    where 
        P: payload::Variant<V>,
        V: std::any::Any + Send,
    {
        let message = self.recv.recv().await
            .map_err(util::discard)?;
        message.into_variant()
            .map_err(util::discard)
    }

    pub fn try_recv<V>(&self) -> Result<V, crate::TempError>
    where 
        P: payload::Variant<V>,
        V: std::any::Any + Send,
    {
        let message = self.recv.try_recv()
            .map_err(util::discard)?;
        message.into_variant()
            .map_err(util::discard)
    }
}