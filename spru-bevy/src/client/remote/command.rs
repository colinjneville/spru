use std::collections::HashMap;

use bevy::prelude;

#[derive(Debug)]
pub struct JoinRemoteBuilder {
    config: aeronet_webtransport::client::ClientConfig,
    url: String,
    headers: HashMap<String, String>,
}

impl JoinRemoteBuilder {
    pub fn new(url: String) -> Self {
        Self {
            config: Default::default(),
            url,
            headers: HashMap::new(),
        }
    }

    /// Add a key-value pair header entry
    /// Note: This is currently implemented using query parameters in the URL, as browser implementations of webtransport
    /// don't currently support adding to the request headers.
    /// This appears to be changing, and implementation may be able to switch to actual headers: 
    /// https://github.com/w3c/webtransport/issues/263
    pub fn with_header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the connection configuration.  
    /// Note: [ClientConfig](aeronet_webtransport::client::ClientConfig) is a different
    /// underlying type on WASM and non-WASM targets.
    pub fn with_config(mut self, config: aeronet_webtransport::client::ClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> Result<JoinRemote, url::ParseError> {
        Ok(JoinRemote {
            url: self.url()?,
            config: self.config,
        })
    }

    /// The base url combined with any header entries as queries.  
    pub fn url(&self) -> Result<url::Url, url::ParseError> {
        url::Url::parse_with_params(&self.url, &self.headers)
    }
}

#[derive(Debug)]
pub struct JoinRemote {
    url: url::Url,
    config: aeronet_webtransport::client::ClientConfig,
}

impl prelude::Command for JoinRemote {
    fn apply(self, world: &mut bevy::ecs::world::World) {
        let Self {
            url,
            config,
        } = self;

        world.commands()
            .spawn_empty()
            .queue(aeronet_webtransport::client::WebTransportClient::connect(config, url));

        world.flush();
    }
}

