//! Errors returned by [Server](crate::Server) operations

use crate::{
    action,
    common::{self, error::FatalError},
    game, player, 
};

#[allow(unused)]
use crate::{common::Seed, Reaction, Server, server::Save};

/// An error occurred during [Server::init]
#[derive(Debug, thiserror::Error)]
#[error("Server::init failed: {0}")]
pub enum InitError {
    /// [game::Init] error
    GameInit(#[from] game::init::Error),
}

/// An error occurred during [Server::save]
#[derive(Debug, thiserror::Error)]
#[error("Server::save failed: {0}")]
pub enum SaveError {
    /// Serialization failure when creating the [Save]
    Snapshot(#[from] common::error::Save),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] common::error::FatalError),
}

common::error::impl_get_fatal_error!(SaveError);

/// An error occurred during [Server::load]
#[derive(Debug, thiserror::Error)]
#[error("Server::load failed: {0}")]
pub enum LoadError {
    /// Deserialization failure when loading the [Save]
    Snapshot(#[from] common::error::Load),
}

/// An error occurred during [Server::add_player]
#[derive(Debug, thiserror::Error)]
#[error("Server::add_player failed: {0}")]
pub enum AddPlayerError {
    /// [player::Init] error
    PlayerInit(#[from] player::init::Error),
    /// Serialization failure while creating [Seed]
    Snapshot(#[from] common::error::Save),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(AddPlayerError);

/// An error occurred during [Server::remove_player]
#[derive(Debug, thiserror::Error)]
#[error("Server::remove_player failed: {0}")]
pub enum RemovePlayerError {
    /// The `player_id` is invalid
    Player(#[from] player::Error),
    /// [player::Init] error
    PlayerRemove(#[from] player::init::Error),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(RemovePlayerError);

/// An error occurred during [Server::deactivate_player]
#[derive(Debug, thiserror::Error)]
#[error("Server::deactivate_player failed: {0}")]
pub enum DeactivatePlayerError {
    /// The `player_id` is invalid
    Player(#[from] player::Error),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(DeactivatePlayerError);

/// An error occurred during [Server::reseed_player]
#[derive(Debug, thiserror::Error)]
#[error("Server::reseed_player failed: {0}")]
pub enum ReseedPlayerError {
    /// The `player_id` is invalid
    Player(#[from] player::Error),
    /// The server failed to snapshot the current state
    Snapshot(#[from] common::error::Save),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(ReseedPlayerError);

/// An error occurred during [Server::manual_trigger]
#[derive(Debug, thiserror::Error)]
#[error("Server::manual_trigger failed: {0}")]
pub enum ManualTriggerError {
    /// [trait@Reaction] error
    Reaction(#[from] action::Error),
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(ManualTriggerError);

/// An error occurred during [Server::signal]
#[derive(Debug, thiserror::Error)]
#[error("Server::signal failed: {0}")]
pub enum SignalError {
    /// Fatal error, the server must be recreated from a [Save]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(SignalError);
