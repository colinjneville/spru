pub mod command;
pub mod component;
pub mod event;
mod plugin;
pub use plugin::Plugin;
pub(crate) mod system;

use std::fmt;

use derive_where::derive_where;

use crate::common;


#[derive(Debug)]
pub enum JoinRequestResponse<PlayerInitIn> {
    AcceptNew(PlayerInitIn),
    AcceptReconnect(spru::player::Id),
    RejectNotFound(String),
    RejectNotAllowed(String),
}

impl<PlayerInitIn> fmt::Display for JoinRequestResponse<PlayerInitIn> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptNew(_) => write!(f, "Accept"),
            Self::AcceptReconnect(player_id) => {
                write!(f, "Accept reconnect: {player_id}")
            }
            Self::RejectNotFound(message) => write!(f, "Reject: Not Found: {message}"),
            Self::RejectNotAllowed(message) => write!(f, "Reject: Not Allowed: {message}"),
        }
    }
}

#[derive_where(Debug; spru::common::Seed<Common>)]
pub struct PendingClient<Common: common::CommonSSS> {
    pub seed: spru::common::Seed<Common>,
}