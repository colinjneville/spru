use std::marker::PhantomData;

use bevy::prelude;

use crate::client;

#[derive(Debug)]
pub struct JoinRemote<Client> {
    config: aeronet_webtransport::client::ClientConfig,
    url: url::Url,
    // headers: HashMap<String, String>,
    _p: PhantomData<Client>,
}

impl<Client> JoinRemote<Client> {
    pub fn new(url: url::Url) -> Self {
        Self {
            config: Default::default(),
            url,
            // headers: HashMap::new(),
            _p: PhantomData,
        }
    }

    /// Add a key-value pair header entry
    /// Note: This is currently implemented using query parameters in the URL, as browser implementations of webtransport
    /// don't currently support adding to the request headers.
    /// This appears to be changing, and implementation may be able to switch to actual headers: 
    /// https://github.com/w3c/webtransport/issues/263
    #[must_use]
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.url.query_pairs_mut()
            .append_pair(key, value);
        // self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a key-value pair header entry
    /// Note: This is currently implemented using query parameters in the URL, as browser implementations of webtransport
    /// don't currently support adding to the request headers.
    /// This appears to be changing, and implementation may be able to switch to actual headers: 
    /// https://github.com/w3c/webtransport/issues/263
    pub fn set_header(&mut self, key: &str, value: &str) {
        self.url.query_pairs_mut()
            .append_pair(key, value);
        // self.headers.insert(key.to_string(), value.to_string());
    }

    /// Set the connection configuration.  
    /// Note: [ClientConfig](aeronet_webtransport::client::ClientConfig) is a different
    /// underlying type on WASM and non-WASM targets.
    #[must_use]
    pub fn with_config(mut self, config: aeronet_webtransport::client::ClientConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the connection configuration.  
    /// Note: [ClientConfig](aeronet_webtransport::client::ClientConfig) is a different
    /// underlying type on WASM and non-WASM targets.
    pub fn set_config(&mut self, config: aeronet_webtransport::client::ClientConfig) {
        self.config = config;
    }

    /// The base url combined with any header entries as queries.  
    pub fn url(&self) -> &url::Url {
        &self.url
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand for JoinRemote<Client> {
    type Out = ();
    
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            url,
            config,
            _p,
        } = self;

        prelude::info!("Requesting connection to server at {url}");
        
        entity
            .insert((
                client::component::FromServer::<Client>::default(),
                client::remote::component::PendingClient::<Client>::default(),
            ));
        aeronet_webtransport::client::WebTransportClient::connect(config, url)
            .apply(entity);
    }
}

