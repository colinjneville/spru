use derive_where::derive_where;

use crate::{client, player};

#[must_use]
#[derive_where(Debug; client::init::Arg<Server::Common>)]
pub struct Ret<Server: super::Server> {
    pub client_init: client::init::Arg<Server::Common>,
}

pub type Result<Server> = std::result::Result<super::Output<Server, Ret<Server>>, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}