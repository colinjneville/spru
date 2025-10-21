use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::common;

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Init<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub result: Result<super::component::ClientId, spru::client::init::Error>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct Signal<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub result: Result<(), spru::client::signal::Error>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; Client::Interaction)]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct StageInteraction<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub interaction: Client::Interaction,
    pub result: Result<spru::transaction::Pending, spru::client::stage_interaction::Error>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct RevertInteractions<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub pending_transaction_id: Option<spru::transaction::Pending>,
    pub result: Result<(), spru::client::revert_interactions::Error>,
    pub(crate) _client: PhantomData<fn() -> Client>,
}

#[derive_where(Debug; )]
#[derive(prelude::Event)]
#[non_exhaustive]
pub struct ApplyInteractions<Client: super::ClientSSS> {
    pub game_id: common::component::GameId,
    pub client_id: super::component::ClientId,
    pub pending_transaction_id: Option<spru::transaction::Pending>,
    pub result: Result<(), spru::client::apply_interactions::Error>,
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