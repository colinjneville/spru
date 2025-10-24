use derive_where::derive_where;

use crate::common;

#[must_use]
#[derive_where(Debug; Server, common::Seed<Server::Common>)]
pub struct Ret<Server: super::Server> {
    pub server: Server,
    pub client_inits: Vec<common::Seed<Server::Common>>,
}
