use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct RemoteClient {
    pub player_id: spru::player::Id,
}