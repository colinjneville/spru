use std::marker::PhantomData;
use std::time;

use bevy::prelude;

use crate::server;

#[derive(Debug)]
pub struct StartListener<Server> {
    config: aeronet_webtransport::server::ServerConfig,
    _p: PhantomData<Server>,
}

impl<Server> StartListener<Server> {
    pub fn new(config: aeronet_webtransport::server::ServerConfig) -> Self {
        Self {
            config,
            _p: PhantomData,
        }
    }

    pub fn new_localhost() -> Self {
        let identity = aeronet_webtransport::wtransport::Identity::self_signed(["localhost", "127.0.0.1", "::1"])
            .unwrap();
        let cert = &identity.certificate_chain().as_slice()[0];
        let spki_fingerprint = aeronet_webtransport::cert::spki_fingerprint_b64(cert)
            .unwrap();
        let cert_hash = aeronet_webtransport::cert::hash_to_b64(cert.hash());
        prelude::info!("Generated localhost server config: \n{spki_fingerprint}\n{cert_hash}");

        let config = aeronet_webtransport::wtransport::ServerConfig::builder()
            .with_bind_default(25576)
            .with_identity(identity)
            .keep_alive_interval(Some(time::Duration::from_secs(5)))
            .max_idle_timeout(Some(time::Duration::from_secs(120)))
            .expect("Max timeout should be before the heat death of the universe")
            .build();

        Self::new(config)
    }
}

impl<Server: server::ServerSSS> prelude::EntityCommand for StartListener<Server> {
    fn apply(self, entity: bevy::ecs::world::EntityWorldMut) {
        if entity.contains::<server::component::Runner<Server>>() {
            aeronet_webtransport::server::WebTransportServer::open(self.config)
                .apply(entity);
        } else {
            prelude::error!("Cannot StartListener on an Entity without spru_bevy::server::component::Runner");
        }
    }
}
