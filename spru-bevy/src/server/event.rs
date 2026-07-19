use std::{marker::PhantomData, sync::Arc};

use bevy::prelude;
use derive_where::derive_where;

use crate::{common, server};



#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Initialized {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InitializeError {
    pub entity: prelude::Entity,
    pub error: spru::server::error::InitError,
}


#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Signaled {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub sender: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct SignalError {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub sender: spru::player::Id,
    pub error: Arc<spru::server::error::SignalError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerAdded {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerAddError {
    pub entity: prelude::Entity,
    pub error: Arc<spru::server::error::AddPlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerRemoved {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerRemoveError {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
    pub error: Arc<spru::server::error::RemovePlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerDeactivated {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerDeactivateError {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
    pub error: Arc<spru::server::error::DeactivatePlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerReseeded {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct PlayerReseedError {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
    pub error: Arc<spru::server::error::ReseedPlayerError>,
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::Trigger)]
#[derive(prelude::EntityEvent)]
pub struct ManualTrigger<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub trigger: <Server::Reaction as spru::Reaction>::Trigger,
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::Trigger)]
#[derive(prelude::EntityEvent)]
pub struct ManualTriggerError<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub trigger: <Server::Reaction as spru::Reaction>::Trigger,
    pub error: Arc<spru::server::error::ManualTriggerError>,
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::GameOutcome)]
#[derive(prelude::EntityEvent)]
pub struct GameCompleted<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub game_outcome: <Server::Reaction as spru::Reaction>::GameOutcome,
}
