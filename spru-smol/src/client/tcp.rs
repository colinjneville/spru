use std::{future::Future, pin::Pin};

use futures_lite::AsyncReadExt;

use crate::error::IoDeserializeError;

pub struct Tcp<Actions> {
    read_buffer: Vec<u8>,
    stream: async_once_cell::Lazy<std::io::Result<smol::net::TcpStream>, Pin<Box<dyn Future<Output = std::io::Result<smol::net::TcpStream>>>>>,
    transactions: smol::channel::Sender<spru::transaction::Confirmed<Actions>>,
}

impl<Actions> Tcp<Actions> {
    pub fn new(addr: impl smol::net::AsyncToSocketAddrs + 'static) -> Self {
        // TODO adjustable max message size
        let read_buffer = vec![0u8; 1024 * 10];
        let stream = async_once_cell::Lazy::<_, Pin<Box<dyn Future<Output = _>>>>::new(Box::pin(async move {
            smol::net::TcpStream::connect(addr).await
        }));

        let (transactions, transactions_recv) = smol::channel::unbounded();
        
        Self {
            read_buffer,
            stream,
            transactions,
        }
    }

    async fn get_stream(&self) -> std::io::Result<smol::net::TcpStream> {
        let stream = self.stream.get_unpin().await.as_ref().map_err(std::io::Error::kind)?;
        Ok(stream.clone())
    }

    pub async fn recv(&mut self) -> Result<(), IoDeserializeError> {
        let stream = self.get_stream().await?;
        // let message: crate::client::Message<Actions> = crate::util::smol::deserialize_over_stream(&mut stream, &mut self.read_buffer)?;
        // match message {
            
        // }
        Ok(())
    }
}