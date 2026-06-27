use std::collections::HashMap;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct RemoteClient {
    pub player_id: spru::player::Id,
}

#[derive_where(Debug; spru::common::Seed<Common>)]
#[derive(prelude::Component)]
pub struct PendingRemote<Common: common::CommonSSS> {
    pub seed: Option<spru::common::Seed<Common>>,
}