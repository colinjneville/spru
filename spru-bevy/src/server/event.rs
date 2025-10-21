use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Init<Server: super::ServerSSS> {
    pub result: Result<common::component::GameId, spru::server::init::Error>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Signal<Server: super::ServerSSS> {
    pub game_id: common::component::GameId,
    pub sender: spru::player::Id,
    pub result: Result<(), spru::server::signal::Error>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct AddPlayer<Server: super::ServerSSS> {
    pub game_id: common::component::GameId,
    pub result: Result<spru::player::Id, spru::server::add_player::Error>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct ManualTrigger<Server: super::ServerSSS> {
    pub game_id: common::component::GameId,
    pub result: Result<(), spru::server::manual_trigger::Error>,
    pub(crate) _server: PhantomData<fn() -> Server>,
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::GameOutcome)]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct GameComplete<Server: super::ServerSSS> {
    pub game_id: common::component::GameId,
    pub game_outcome: <Server::Reaction as spru::Reaction>::GameOutcome,
}