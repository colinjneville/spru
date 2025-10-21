use derive_where::derive_where;

use crate::{client, common};

#[must_use]
#[derive_where(Debug; Server, client::init::Arg<Server::Common>)]
pub struct Ret<Server: super::Server> {
    pub server: Server,
    pub client_inits: Vec<client::init::Arg<Server::Common>>,
}

pub type Result<Server> = std::result::Result<Ret<Server>, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Snapshot(#[from] common::error::Load),
}