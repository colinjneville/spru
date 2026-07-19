use std::collections::HashMap;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship(relationship_target = Listener)]
pub struct ListenerFor(pub prelude::Entity);

impl ListenerFor {
    pub fn listener_for(&self) -> prelude::Entity {
        self.0
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship_target(relationship = ListenerFor)]
pub struct Listener(prelude::Entity);

impl Listener {
    pub fn listener(&self) -> prelude::Entity {
        self.0
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct RemoteClient {
    #[reflect(remote = crate::reflect::spru::player::Id)]
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship(relationship_target = RemoteClients)]
pub struct RemoteClientFor(pub prelude::Entity);

impl RemoteClientFor {
    pub fn remote_client_for(&self) -> prelude::Entity {
        self.0
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[relationship_target(relationship = RemoteClientFor)]
pub struct RemoteClients(Vec<prelude::Entity>);

impl RemoteClients {
    pub fn remote_clients(&self) -> &[prelude::Entity] {
        &self.0
    }
}

#[derive_where(Debug; spru::common::Seed<Common>)]
#[derive(prelude::Component)]
pub struct PendingClient<Common: common::CommonSSS> {
    pub seed: Option<spru::common::Seed<Common>>,
}

#[derive(Debug, Clone)]
#[derive(prelude::Component, prelude::Reflect)]
#[reflect(opaque)]
#[component(immutable)]
pub struct Certificate(aeronet_webtransport::wtransport::tls::CertificateChain);

impl Certificate {
    pub(crate) fn new(chain: aeronet_webtransport::wtransport::tls::CertificateChain) -> Self {
        Self(chain)
    }

    pub fn hash(&self) -> crate::remote::component::CertificateHash {
        let hash = self.0.as_slice()[0].hash().as_ref().clone();
        crate::remote::component::CertificateHash(hash)
    }

    pub fn spki_fingerprint(&self) -> SpkiFingerprint {
        let cert = &self.0.as_slice()[0];
        let spki_fingerprint = aeronet_webtransport::cert::spki_fingerprint(cert)
            .expect("Identity must be invalid");

        SpkiFingerprint(spki_fingerprint)
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[component(immutable)]
pub struct SpkiFingerprint(pub [u8; 32]);

impl SpkiFingerprint {
    pub fn to_base64(&self) -> String {
        crate::u256_to_base64(&self.0)
    }
}