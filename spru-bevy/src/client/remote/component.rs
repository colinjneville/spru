use std::{collections::HashMap, marker::PhantomData};

use bevy::prelude;
use derive_where::derive_where;
use url::Url;

use crate::reflect;

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship(relationship_target = Remote)]
pub struct RemoteFor(#[relationship] pub prelude::Entity);

impl RemoteFor {
    pub fn remote_for(&self) -> prelude::Entity {
        self.0
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship_target(relationship = RemoteFor)]
pub struct Remote(prelude::Entity);

impl Remote {
    pub fn remote(&self) -> prelude::Entity {
        self.0
    }
}

#[derive_where(Debug, Default; )]
#[derive(prelude::Component, prelude::Reflect)]
pub struct PendingClient<Client>(PhantomData<Client>);

#[derive(Debug, Clone)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct ConnectionConfig {
    /// Address with port
    #[reflect(remote = reflect::url::Url)]
    pub address: Url,
    pub certificate_hash: Option<crate::remote::component::CertificateHash>,
    pub headers: HashMap<String, String>,
}

impl ConnectionConfig {
    /// Create a default config for a given URL. The URL
    /// should include a port.
    pub fn new(address: url::Url) -> Self {
        Self {
            address,
            certificate_hash: None,
            headers: HashMap::new(),
        }
    }

    /// Create a default config for localhost.
    pub fn new_localhost(port: u16) -> Self {
        Self {
            address: url::Url::parse(&format!("https://localhost:{port}")).unwrap(),
            certificate_hash: None,
            headers: HashMap::new(),
        }
    }
}

impl ConnectionConfig {
    pub(crate) fn url(&self) -> url::Url {
        let mut url = self.address.clone();
        for (k, v) in &self.headers {
            url.query_pairs_mut().append_pair(k, v);
        }
        url
    }

    pub(crate) fn client_config(&self) -> aeronet_webtransport::client::ClientConfig {
        let config = cfg_select! {
            all(target_family = "wasm", target_os = "unknown") => {
                {
                    let server_certificate_hashes = if let Some(certificate_hash) = self.certificate_hash.as_ref() {
                        vec![aeronet_webtransport::xwt_web::CertificateHash {
                            algorithm: aeronet_webtransport::xwt_web::HashAlgorithm::Sha256,
                            value: certificate_hash.0.to_vec(),
                        }]
                    } else {
                        vec![]
                    };
                    
                    aeronet_webtransport::xwt_web::WebTransportOptions {
                        server_certificate_hashes,
                        .. Default::default()
                    }
                }
            }
            _ => {
                {
                    let config = aeronet_webtransport::client::ClientConfig::builder()
                        .with_bind_default();

                    if let Some(cert_hash) = self.certificate_hash.as_ref() {
                        config.with_server_certificate_hashes([aeronet_webtransport::wtransport::tls::Sha256Digest::new(cert_hash.0)])
                    } else {
                        config.with_native_certs()
                    }.build()
                }
            }
        };

        config
    }
}
