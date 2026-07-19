use bevy::prelude;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct CertificateHash(pub [u8; 32]);

impl CertificateHash {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn to_base64(&self) -> String {
        let hash = &self.0;

        let mut s = vec![0u8; 44];

        let (chunks, remainder) = hash.as_chunks::<3>();
        let padded_chunk = [remainder[0], remainder[1], 0];
        
        for (i, &[a, b, c]) in chunks.iter().chain(std::iter::once(&padded_chunk)).enumerate() {
            s[i * 4] = a >> 2;
            s[i * 4 + 1] = (a << 6 >> 2) | (b >> 4);
            s[i * 4 + 2] = (b << 4 >> 2) | (c >> 6);
            s[i * 4 + 3] = c << 2 >> 2;
        }
        for c in &mut s {
            *c = match *c {
                0..26 => {
                    *c + b'A'
                }
                26..52 => {
                    *c + b'a' - 26
                }
                52..62 => {
                    *c + b'0' - 52
                }
                62 => {
                    *c + b'+' - 62
                }
                63.. => {
                    *c + b'/' - 63
                }
            };
        }
        s[43] = b'=';

        String::from_utf8(s).unwrap()
    }
}

impl AsRef<[u8; 32]> for &CertificateHash {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for CertificateHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.to_base64();
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_encode_base64() {
        let cert_hash = super::CertificateHash([0; _]);
        assert_eq!(&cert_hash.to_base64(), "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
        let cert_hash = super::CertificateHash([0x75, 0xca, 0xc5, 0x52, 0x00, 0x16, 0x30, 0x36, 0x63, 0xc6, 0x2c, 0xc2, 0x46, 0xf4, 0x68, 0x85, 0x21, 0xf4, 0x66, 0x4e, 0xe5, 0x96, 0xdf, 0x3d, 0xe4, 0x80, 0x4c, 0x42, 0x10, 0x5c, 0x74, 0x60]);
        assert_eq!(&cert_hash.to_base64(), "dcrFUgAWMDZjxizCRvRohSH0Zk7llt895IBMQhBcdGA=");
    }
}
