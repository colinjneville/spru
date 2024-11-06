use std::{any, marker::PhantomData};

use serde::Serialize;
use spru_message::{header, payload, Header, Message};

use crate::util;


#[derive(Debug)]
pub(crate) enum Route<P> {
    Local(Local<P>),
    Tcp(Tcp<P>),
}

impl<P> Route<P> {
    pub async fn send<V>(&self, data: V) -> Result<(), V> 
    where 
        V: any::Any + Serialize + Send,
        P: payload::Variant<V>,
    {
        match self {
            Self::Local(l) => l.send(data).await,
            Self::Tcp(t) => 
                t.send(&data).await
                    .map_err(|_| data),
        }
    }

    pub fn send_blocking<V>(&self, data: V) -> Result<(), V> 
    where 
        V: any::Any + Serialize + Send,
        P: payload::Variant<V>,
    {
        match self {
            Self::Local(l) => l.send_blocking(data),
            Self::Tcp(t) => 
                t.send_blocking(&data)
                    .map_err(|_| data),
        }
    }

    pub async fn close(&self) -> Result<(), crate::TempError> {
        match self {
            Route::Local(l) => l.close().await,
            Route::Tcp(t) => t.close().await,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Tcp<P> {
    addr: std::net::SocketAddr,
    tcp_stream: smol::net::TcpStream,
    _p: PhantomData<fn(P) -> P>,
}

impl<P> Tcp<P> {
    const MAX_MESSAGE_LEN: usize = 2048;

    pub(crate) fn new(addr: std::net::SocketAddr, tcp_stream: smol::net::TcpStream) -> Self {
        Self {
            addr,
            tcp_stream: tcp_stream,
            _p: Default::default()
        }
    }

    pub fn send_blocking<V>(&self, data: &V) -> Result<(), crate::TempError>
    where
        P: payload::Variant<V>,
        V: Serialize,
    {
        smol::block_on(self.send(data))
    }

    pub async fn send<V>(&self, data: &V) -> Result<(), crate::TempError> 
    where
        P: payload::Variant<V>,
        V: Serialize,
    {
        let message = Message::<P>::new_serialized(data)
            .map_err(util::discard)?;
        // Message is both the header and payload
        if message.header.payload_size > util::PAYLOAD_MAX_LEN {
            return Err(crate::TempError);
        }

        use smol::io::AsyncWriteExt as _;

        let mut tcp_stream = self.tcp_stream.clone();
        tcp_stream.write_all(&*message.into_bytes()).await
            .map_err(util::discard)?;

        tcp_stream.flush().await
            .map_err(util::discard)?;

        Ok(())
    }

    pub async fn close(&self) -> Result<(), crate::TempError> {
        use futures_lite::AsyncWriteExt as _;
        let mut tcp_stream = self.tcp_stream.clone();
        tcp_stream.close().await
            .map_err(util::discard)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Local<P> {
    send: smol::channel::Sender<Message<payload::Raw<P>>>,
}

impl<P> Local<P> {
    pub(crate) fn new(send: smol::channel::Sender<Message<payload::Raw<P>>>) -> Self {
        Self {
            send,
        }
    }

    pub async fn send<V>(&self, data: V) -> Result<(), V> 
    where 
        V: std::any::Any + Send,
        P: payload::Variant<V>,
    {
        let message = spru_message::Message::<P>::new_raw(data);

        // Channel is unbounded, and can never be full
        self.send.force_send(message)
            .map(util::discard)
            .map_err(|e| {
                let Ok(v) = e.0.into_variant() else { panic!("recast to original type must succeed")};
                v
            })
    }

    pub fn send_blocking<V>(&self, data: V) -> Result<(), V> 
    where 
        V: std::any::Any + Send,
        P: payload::Variant<V>,
    {
        let message = spru_message::Message::<P>::new_raw(data);

        // Channel is unbounded, and can never be full
        self.send.force_send(message)
            .map(util::discard)
            .map_err(|e| {
                let Ok(v) = e.0.into_variant() else { panic!("recast to original type must succeed")};
                v
            })
    }

    pub async fn close(&self) -> Result<(), crate::TempError> {
        self.send.close();
        Ok(())
    }
}