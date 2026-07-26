#[derive(Debug)]
struct ConfigRemoteJoinPanel {
    address: Validated<url::Url>,
    port: Validated<u16>,
    cert_hash: Validated<BlankableCertificateHash>,
    password: String,
}

impl Default for ConfigRemoteJoinPanel {
    fn default() -> Self {
        Self { 
            address: Validated::new(url::Url::parse("https://localhost").unwrap()), 
            port: Validated::new(super::DEFAULT_PORT), 
            cert_hash: Validated::new(BlankableCertificateHash::default()),
            password: String::new(),
        }
    }
}

impl ConfigRemoteJoinPanel {
    fn validate_address(s: &str) -> Result<url::Url, &'static str> {
        let mut url = url::Url::parse(s)
            .map_err(|_| "Invalid address")?;

        // Test if we can set a port
        url.set_port(url.port())
            .map_err(|_| "Invalid address")?;

        Ok(url)
    }

    fn validate_hash(s: &str) -> Result<BlankableCertificateHash, &'static str> {
        if s.is_empty() {
            Ok(BlankableCertificateHash::default())
        } else {
            spru_bevy::remote::aeronet_webtransport::cert::hash_from_b64(s)
                .map(CertificateHash)
                .map(BlankableCertificateHash)
                .map_err(|_| "Invalid certificate hash")
        }
    }
}

impl super::ConfigPanel for ConfigRemoteJoinPanel {
    

    fn valid(&self) -> Result<(), std::borrow::Cow<'static, str>> {
        Ok(())
    }
}
