use std::sync::Arc;

use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct LocalPlayerAdded {
    #[event_target]
    pub server_entity: prelude::Entity,
    pub client_entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct LocalPlayerAddError {
    #[event_target]
    pub server_entity: prelude::Entity,
    pub error: Arc<spru::server::error::AddPlayerError>,
}