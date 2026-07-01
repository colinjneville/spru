use std::collections::HashMap;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct RemoteClient {
    #[reflect(remote = crate::reflect::spru::player::Id)]
    pub player_id: spru::player::Id,
}

#[derive_where(Debug; spru::common::Seed<Common>)]
#[derive(prelude::Component)]
pub struct PendingClient<Common: common::CommonSSS> {
    pub seed: Option<spru::common::Seed<Common>>,
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[component(immutable)]
pub struct Certificate {
    pub hash: [u8; 32],
    pub spki_fingerprint: [u8; 32],
}