use derive_where::derive_where;

use crate::{client, player};

#[derive_where(Debug, Serialize, Deserialize; <Server::PlayerInit as crate::player::Init>::In)]
pub struct Arg<Server: super::Server> {
    pub init_input: <Server::PlayerInit as crate::player::Init>::In,
}

#[must_use]
#[derive_where(Debug; client::init::Arg<Server::Common>)]
pub struct Ret<Server: super::Server> {
    pub client_init: client::init::Arg<Server::Common>,
    pub player_id: player::Id,
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}