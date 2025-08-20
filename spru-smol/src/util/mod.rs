mod spawner;
pub use spawner::{LocalExecutorSpawner, RemoteExecutorSpawner};

use std::pin::Pin;

use crate::error::{DeserializeError, IoDeserializeError, IoSerializeError, SerializeError};

pub(crate) fn discard<T, U: Default>(_t: T) -> U {
    U::default()
}

// TODO
pub(crate) const PAYLOAD_MAX_LEN: usize = 2048usize;

pub async fn serialize_over_stream<S: serde::Serialize>(mut stream: impl smol::io::AsyncWrite + std::marker::Unpin, buffer: &mut [u8], data: S) -> Result<usize, IoSerializeError> {
    use smol::io::AsyncWriteExt;

    let cursor = std::io::Cursor::new(buffer);

    let mut serializer = rmp_serde::Serializer::new(cursor);
    data.serialize(&mut serializer).map_err(SerializeError::new)?;
    let cursor = serializer.into_inner();

    let len = cursor.position() as usize;

    stream.write(&len.to_le_bytes()).await?;

    let buffer = &mut cursor.into_inner()[0..len];
    
    let bytes_sent = stream.write(buffer).await?;
    stream.flush().await?;

    Ok(bytes_sent)
}

pub async fn deserialize_over_stream<S: serde::de::DeserializeOwned>(mut stream: impl smol::io::AsyncRead + std::marker::Unpin, buffer: &mut [u8]) -> Result<S, IoDeserializeError> {
    use smol::io::AsyncReadExt;

    let max_len = buffer.len();

    let mut size_buf = [0u8; 8];
    stream.read_exact(&mut size_buf).await?;
    let size = u64::from_le_bytes(size_buf) as usize;
    
    if size > max_len {
        return Err(IoDeserializeError::Io(std::io::ErrorKind::InvalidData.into()));
    }

    stream.read_exact(&mut buffer[0..size]).await?;

    let data = S::deserialize(&mut rmp_serde::Deserializer::new(&*buffer)).map_err(DeserializeError::new)?;

    Ok(data)
}

pub fn duplex_stream() -> (DuplexStream, DuplexStream) {
    let cursor0 = futures_lite::io::Cursor::new(vec![]);
    let (read0, write0) = smol::io::split(cursor0);
    let cursor1 = futures_lite::io::Cursor::new(vec![]);
    let (read1, write1) = smol::io::split(cursor1);

    (
        DuplexStream {
            read: read0,
            write: write1,
        },
        DuplexStream {
            read: read1,
            write: write0,
        }
    )
}

pub struct DuplexStream {
    read: smol::io::ReadHalf<futures_lite::io::Cursor<Vec<u8>>>,
    write: smol::io::WriteHalf<futures_lite::io::Cursor<Vec<u8>>>,
}

impl DuplexStream {
    fn pinned_read(self: Pin<&mut Self>) -> Pin<&mut smol::io::ReadHalf<futures_lite::io::Cursor<Vec<u8>>>> {
        unsafe {
            self.map_unchecked_mut(|s| &mut s.read)
        }
    }

    fn pinned_write(self: Pin<&mut Self>) -> Pin<&mut smol::io::WriteHalf<futures_lite::io::Cursor<Vec<u8>>>> {
        unsafe {
            self.map_unchecked_mut(|s| &mut s.write)
        }
    }
}

use smol::io::{AsyncRead, AsyncWrite};

impl AsyncRead for DuplexStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.pinned_read().poll_read(cx, buf)
    }
    
    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.pinned_read().poll_read_vectored(cx, bufs)
    }
}

impl AsyncWrite for DuplexStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.pinned_write().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        self.pinned_write().poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        self.pinned_write().poll_close(cx)
    }
    
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.pinned_write().poll_write_vectored(cx, bufs)
    }
}


#[cfg(test)]
mod test {
    use spru_util::verbatim;
    use tagset::tagset;

    use super::*;

    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(impl spru::action::Base)]
    #[tagset(impl<Lookup: spru::item::Lookup> spru::Action<Lookup>)]
    #[tagset(include(spru_util::verbatim::Actions<i32>))]
    #[tagset(include(spru_util::verbatim::Actions<i64>))]
    struct Actions;

    #[test]
    fn serialize_stream_roundtrip() {
        let value = 4i64;
        let action = verbatim::create(value);
        let message: Actions = action.into();

        let mut buffer = vec![0u8; crate::util::PAYLOAD_MAX_LEN];

        let mut stream = vec![];
        let _ = smol::future::block_on(
            serialize_over_stream(&mut stream, &mut buffer, &message)
        ).unwrap();
        
        let new_message: Actions = smol::future::block_on(
            deserialize_over_stream(&*stream, &mut *buffer)
        ).unwrap();

        let Ok(new_action): Result<verbatim::Create<i64>, _> = new_message.try_into() else { panic!() };

        let (new_value, _) = spru::action::Create::create(&new_action).unwrap();

        assert_eq!(value, new_value);
    }
}