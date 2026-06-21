pub mod component;
pub mod event;
pub mod observer;
mod plugin;
use std::fmt;

pub use plugin::Plugin;
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