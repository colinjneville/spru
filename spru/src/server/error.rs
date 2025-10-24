use crate::{action, common::{self, error::FatalError}, game, player};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InitError {
    #[error("{0}")]
    GameInit(#[from] game::init::Error),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum SaveError {
    #[error(transparent)]
    Snapshot(#[from] common::error::Save),
    #[error(transparent)]
    Fatal(#[from] common::error::FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Snapshot(#[from] common::error::Load),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum AddPlayerError {
    #[error("{0}")]
    PlayerInit(player::init::Error),
    #[error(transparent)]
    Snapshot(#[from] common::error::Save),
    #[error(transparent)]
    Fatal(#[from] FatalError),
}

impl From<player::init::Error> for AddPlayerError {
    fn from(value: player::init::Error) -> Self {
        Self::PlayerInit(value)
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ManualTriggerError {
    #[error("{0}")]
    Reaction(#[from] action::Error),
    #[error(transparent)]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum SignalError {
    #[error(transparent)]
    Fatal(#[from] FatalError),
}
