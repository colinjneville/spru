mod id;
use std::fmt;

pub use id::Id;
pub mod init;
pub use init::Init;
pub(crate) mod manager;
pub(crate) use manager::Manager;

/// The status of a player
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Status {
    /// A player that has been removed from the game by [Server::remove_player](crate::Server::remove_player). This is a permanent status change,
    /// although a game may support rejoining as a new player.
    Removed,
    /// A player that has disconnected or is otherwise currently unable to receive signals.
    /// No signals will be generated for this player.
    /// A player enters this state via [Server::deactivate_player](crate::Server::deactivate_player),
    /// and leaves by [Server::remove_player](crate::Server::remove_player) or [Server::reseed_player](crate::Server::reseed_player). 
    Inactive,
    /// The default status for players.  
    /// Players leave this state via [Server::remove_player](crate::Server::remove_player) or [Server::deactivate_player](crate::Server::deactivate_player).
    Active,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Status::Removed => "Removed",
            Status::Inactive => "Inactive",
            Status::Active => "Active",
        };
        write!(f, "{name}")
    }
}

impl Status {
    pub(crate) fn is_removed(&self) -> bool {
        match self {
            Self::Removed => true,
            _ => false,
        }
    }

    pub(crate) fn is_inactive(&self) -> bool {
        match self {
            Self::Inactive => true,
            _ => false,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        match self {
            Self::Active => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Player {player_id} does not exist")]
    DoesNotExist { player_id: Id },
    #[error("Player {player_id} is in state {invalid_status}, which is not valid for the operation")]
    InvalidStatus { player_id: Id, invalid_status: Status, }
}
