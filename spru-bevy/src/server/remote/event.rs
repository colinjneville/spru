use std::{collections, sync::Arc};

use bevy::prelude;
use derive_where::derive_where;

use crate::{remote, server};

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct ConnectionAttempted<PlayerInitIn> {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The newly-created remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub(crate) response: Option<super::JoinRequestResponse<PlayerInitIn>>,
}

impl<PlayerInitIn> ConnectionAttempted<PlayerInitIn> {
    pub fn set_response(&mut self, response: super::JoinRequestResponse<PlayerInitIn>) {
        self.response = Some(response);
    }
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct ConnectionAccepted {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct ReconnectionAccepted {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct ConnectionRejected {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub message: String,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerAdded {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,

    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerAddError {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,

    pub error: Arc<spru::server::error::AddPlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerReseeded {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,

    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerReseedError {
    #[event_target]
    pub server_entity: prelude::Entity,
    // The remote client entity, which is a child of the server (`entity`).
    pub client_entity: prelude::Entity,

    pub error: Arc<spru::server::error::ReseedPlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerRemoved {
    #[event_target]
    pub server_entity: prelude::Entity,
    pub client_entity: prelude::Entity,
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerRemoveError {
    #[event_target]
    pub server_entity: prelude::Entity,
    pub client_entity: prelude::Entity,
    pub player_id: spru::player::Id,
    pub error: Arc<spru::server::error::RemovePlayerError>,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct RemotePlayerDisconnected {
    #[event_target]
    pub server_entity: prelude::Entity,
    pub client_entity: prelude::Entity,
    pub player_id: spru::player::Id,
    pub reason: remote::DisconnectedReason,
}