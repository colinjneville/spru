use std::marker::PhantomData;
use std::time;

use bevy::prelude;

use crate::server;

#[derive(Debug, Default)]
pub struct StartListenerBuilder {
    pub identity: Option<aeronet_webtransport::wtransport::Identity>,
    pub port: Option<u16>,
    pub keep_alive_interval: Option<time::Duration>,
    pub max_idle_timeout: Option<time::Duration>,
}

impl StartListenerBuilder {
    /// A new [StartListener] command builder. Defaults to localhost:52152
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the [StartListener] command
    pub fn build<Server>(self) -> StartListener<Server> {
        let Self {
            identity,
            port,
            keep_alive_interval,
            max_idle_timeout,
        } = self;
        let identity = identity.unwrap_or_else(Self::localhost_identity);
        let port = port.unwrap_or(52152);

        // First certificate should be ours
        let cert = &identity.certificate_chain().as_slice()[0];
        let spki_fingerprint = aeronet_webtransport::cert::spki_fingerprint(cert)
            .expect("Identity must be invalid");

        let certificate = server::remote::component::Certificate {
            hash: cert.hash().as_ref().clone(),
            spki_fingerprint,
        };

        let config = aeronet_webtransport::wtransport::ServerConfig::builder()
            .with_bind_default(port)
            .with_identity(identity)
            .keep_alive_interval(keep_alive_interval)
            .max_idle_timeout(max_idle_timeout)
            .expect("Max timeout should be before the heat death of the universe")
            .build();

        StartListener {
            config,
            certificate,
            _p: PhantomData,
        }
    }

    fn localhost_identity() -> aeronet_webtransport::wtransport::Identity {
        aeronet_webtransport::wtransport::Identity::self_signed(["localhost", "127.0.0.1", "::1"])
            .unwrap()
    }
}

#[derive(Debug)]
pub struct StartListener<Server> {
    config: aeronet_webtransport::server::ServerConfig,
    certificate: server::remote::component::Certificate,
    _p: PhantomData<Server>,
}

impl<Server: server::ServerSSS> prelude::EntityCommand for StartListener<Server> {
    type Out = ();
    
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) {
        let Self {
            config,
            certificate,
            _p,
        } = self;

        if entity.contains::<server::component::Runner<Server>>() {
            entity.insert(certificate);
            aeronet_webtransport::server::WebTransportServer::open(config)
                .apply(entity);
        } else {
            prelude::error!("Cannot StartListener on an Entity without spru_bevy::server::component::Runner");
        }
    }
}
