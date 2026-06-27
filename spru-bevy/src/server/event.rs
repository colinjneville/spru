use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::{common, server};



#[derive_where(Debug; )]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct Init<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub result: Result<common::component::GameId, spru::server::error::InitError>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct Signal<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub sender: spru::player::Id,
    pub result: Result<(), spru::server::error::SignalError>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct AddPlayer<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub result: Result<spru::player::Id, spru::server::error::AddPlayerError>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct ManualTrigger<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub result: Result<(), spru::server::error::ManualTriggerError>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::GameOutcome)]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct GameComplete<Server: server::ServerSSS> {
    pub entity: prelude::Entity,
    pub game_outcome: <Server::Reaction as spru::Reaction>::GameOutcome,
}
