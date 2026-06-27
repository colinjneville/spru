pub mod command;
pub mod component;
pub mod event;
pub mod observer;
mod plugin;
use std::fmt;

use derive_where::derive_where;
pub use plugin::Plugin;

use crate::common;
pub mod system;

#[derive(Debug)]
pub enum JoinRequestResponse<PlayerInitIn> {
    AcceptNew(PlayerInitIn),
    AcceptReconnect(spru::player::Id),
    RejectNotFound,
    RejectNotAllowed,
}

impl<PlayerInitIn> fmt::Display for JoinRequestResponse<PlayerInitIn> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s_accept_reconnect;
        let s = match self {
            Self::AcceptNew(_) => "Accept",
            Self::AcceptReconnect(player_id) => {
                s_accept_reconnect = format!("Accept reconnect: {player_id}");
                &s_accept_reconnect
            }
            Self::RejectNotFound => "Reject: Not Found",
            Self::RejectNotAllowed => "Reject: Not Allowed",
        };
        
        write!(f, "{s}")
    }
}

#[derive_where(Debug; spru::common::Seed<Common>)]
pub struct PendingClient<Common: common::CommonSSS> {
    pub seed: spru::common::Seed<Common>,
}