use std::sync::Arc;

use bevy::prelude;
use derive_where::derive_where;

use crate::{client, common};

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Initialized {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub client_id: client::component::ClientId,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InitializeError {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub error: Arc<spru::client::error::InitError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Signal {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub client_id: client::component::ClientId,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct SignalError {
    pub entity: prelude::Entity,
    pub game_id: common::component::GameId,
    pub client_id: client::component::ClientId,
    pub error: Arc<spru::client::error::SignalError>,
}

#[derive_where(Debug; Client::Interaction)]
#[derive(prelude::EntityEvent)]
pub struct InteractionStaged<Client: client::ClientSSS> {
    pub entity: prelude::Entity,
    pub interaction: Client::Interaction,
    pub pending_id: spru::interaction::Pending,
}

#[derive_where(Debug; Client::Interaction)]
#[derive(prelude::EntityEvent)]
pub struct InteractionStageError<Client: client::ClientSSS> {
    pub entity: prelude::Entity,
    pub interaction: Client::Interaction,
    pub error: Arc<spru::client::error::StageInteractionError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InteractionsReverted {
    pub entity: prelude::Entity,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub count: usize,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InteractionsRevertError {
    pub entity: prelude::Entity,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub error: Arc<spru::client::error::RevertInteractionError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InteractionsApplied {
    pub entity: prelude::Entity,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub count: usize,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct InteractionsApplyError {
    pub entity: prelude::Entity,
    pub pending_interaction_id: Option<spru::interaction::Pending>,
    pub error: Arc<spru::client::error::ApplyInteractionError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct PlayerAdded {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct PlayerRemoved {
    pub entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive_where(Debug; Client::GameOutcome)]
#[derive(prelude::EntityEvent)]
#[non_exhaustive]
pub struct GameComplete<Client: client::ClientSSS> {
    pub entity: prelude::Entity,
    pub game_outcome: Client::GameOutcome,
}
