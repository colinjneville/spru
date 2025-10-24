use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Init<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub result: Result<super::component::ClientId, spru::client::error::InitError>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Signal<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub result: Result<(), spru::client::error::SignalError>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; Client::Interaction)]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct StageInteraction<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub interaction: Client::Interaction,
    pub result: Result<spru::interaction::Pending, spru::client::error::StageInteractionError>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct RevertInteractions<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub result: Result<(), spru::client::error::RevertInteractionError>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct ApplyInteractions<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub result: Result<(), spru::client::error::ApplyInteractionError>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; Client::GameOutcome)]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct GameOutcome<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub game_outcome: Client::GameOutcome,
}