use std::{any, marker::PhantomData};

use crate::util;


#[derive(Debug)]
pub(crate) enum Route<P> {
    Local(Local<P>),
    Tcp(Tcp<P>),
}

impl<P> Route<P> {
    pub async fn send(&self, payload: P) -> Result<(), crate::TempError> 
    where 
        P: any::Any + Send + serde::Serialize,
    {
        match self {
            Self::Local(l) => l.send(payload).await,
            Self::Tcp(t) => 
                t.send(payload).await
                    .map_err(util::discard),
        }
    }

    pub fn send_blocking(&self, payload: P) -> Result<(), crate::TempError> 
    where 
        P: any::Any + Send + serde::Serialize,
    {
        match self {
            Self::Local(l) => l.send_blocking(payload),
            Self::Tcp(t) => 
                t.send_blocking(payload)
                    .map_err(util::discard),
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
            _p: PhantomData,
        }
    }

    pub fn send_blocking<'p>(&self, payload: P) -> Result<(), crate::TempError>
    where
        P: serde::Serialize,
    {
        smol::block_on(self.send(payload))
    }

    pub async fn send<'p>(&self, payload: P) -> Result<(), crate::TempError> 
    where
        P: serde::Serialize,
    {
        let mut serial_buffer = vec![0u8; Self::MAX_MESSAGE_LEN];

        let mut tcp_stream = self.tcp_stream.clone();
        util::serialize_over_stream(&mut tcp_stream, &mut *serial_buffer, &payload)
            .await
            .map_err(util::discard)?;
        
        smol::io::AsyncWriteExt::flush(&mut tcp_stream)
            .await
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
    send: smol::channel::Sender<P>,
}

impl<P> Local<P> {
    pub(crate) fn new(send: smol::channel::Sender<P>) -> Self {
        Self {
            send,
        }
    }

    pub async fn send<'p>(&self, payload: P) -> Result<(), crate::TempError> 
    where 
        P: std::any::Any + Send,
    {
        // Channel is unbounded, and can never be full
        self.send.force_send(payload)
            .map(util::discard)
            .map_err(util::discard)
    }

    pub fn send_blocking<'p>(&self, payload: P) -> Result<(), crate::TempError> 
    where 
        P: std::any::Any + Send,
    {
        // Channel is unbounded, and can never be full
        self.send.force_send(payload.into())
            .map(util::discard)
            .map_err(util::discard)
    }

    pub async fn close(&self) -> Result<(), crate::TempError> {
        self.send.close();
        Ok(())
    }
}