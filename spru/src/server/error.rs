//! Errors returned by [Server](crate::Server) operations

use crate::{
    action,
    common::{self, error::FatalError},
    game, player,
};

/// An error occurred during [Server::init](crate::Server::init)
#[derive(Debug, thiserror::Error)]
#[error("Server::init failed: {0}")]
pub enum InitError {
    /// [game::Init] error
    GameInit(#[from] game::init::Error),
}

/// An error occurred during [Server::save](crate::Server::save)
#[derive(Debug, thiserror::Error)]
#[error("Server::save failed: {0}")]
pub enum SaveError {
    /// Serialization failure when creating the [Save](crate::server::Save)
    Snapshot(#[from] common::error::Save),
    /// Fatal error, the server must be recreated from a [Save](crate::server::Save)
    Fatal(#[from] common::error::FatalError),
}

/// An error occurred during [Server::load](crate::Server::load)
#[derive(Debug, thiserror::Error)]
#[error("Server::load failed: {0}")]
pub enum LoadError {
    /// Deserialization failure when loading the [Save](crate::server::Save)
    Snapshot(#[from] common::error::Load),
}

/// An error occurred during [Server::add_player](crate::Server::add_player)
#[derive(Debug, thiserror::Error)]
#[error("Server::add_player failed: {0}")]
pub enum AddPlayerError {
    /// [player::Init] error
    PlayerInit(#[from] player::init::Error),
    /// Serialization failure while creating [Seed](common::Seed)
    Snapshot(#[from] common::error::Save),
    /// Fatal error, the server must be recreated from a [Save](crate::server::Save)
    Fatal(#[from] FatalError),
}

/// An error occurred during [Server::manual_trigger](crate::Server::manual_trigger)
#[derive(Debug, thiserror::Error)]
#[error("Server::manual_trigger failed: {0}")]
pub enum ManualTriggerError {
    /// [Reaction](trait@crate::Reaction) error
    Reaction(#[from] action::Error),
    /// Fatal error, the server must be recreated from a [Save](crate::server::Save)
    Fatal(#[from] FatalError),
}

/// An error occurred during [Server::signal](crate::Server::signal)
#[derive(Debug, thiserror::Error)]
#[error("Server::signal failed: {0}")]
pub enum SignalError {
    /// Fatal error, the server must be recreated from a [Save](crate::server::Save)
    Fatal(#[from] FatalError),
}
