#[derive(Debug)]
#[derive(deku::DekuRead, deku::DekuWrite)]
#[deku(endian = "big")]
pub struct Header {
    // Length in bytes of payload. 
    pub payload_size: usize,
}

impl Header {
    pub fn byte_length() -> usize {
        use std::sync::atomic;
        static BYTE_LENGTH: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

        let mut len = BYTE_LENGTH.load(atomic::Ordering::Relaxed);
        if len == 0 {
            let mut buffer = vec![];
            while let Err(deku::DekuError::Incomplete(need_size)) = Self::try_from(buffer.as_slice()) {
                buffer.resize(buffer.len() + need_size.byte_size(), 0);
            }

            len = buffer.len();
            BYTE_LENGTH.store(len, atomic::Ordering::Relaxed);
        }

        len
    }

    

    pub fn to_bytes(&self) -> Box<[u8]> {
        deku::DekuContainerWrite::to_bytes(self)
            .expect("Serialization is infallible")
            .into_boxed_slice()
    }

    pub fn from_bytes(&self, bytes: &[u8]) -> Result<Self, Error> {
        match Self::try_from(bytes) {
            Ok(header) => Ok(header),
            Err(deku::DekuError::Incomplete(_) ) => todo!(),
            Err(_) => Err(Error::Invalid),
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("The header is incomplete")]
    Incomplete,
    #[error("The header is invalid")]
    Invalid,
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn header_size() {
        let a = Header::byte_length();
        assert_ne!(a, 0);
        let b = Header::byte_length();
        assert_eq!(a, b);
    }
}