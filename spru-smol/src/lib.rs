pub mod client;
pub mod error;
pub mod game;
pub use game::Server;
pub mod lobby;
pub use lobby::Lobby;
pub mod router;
pub use router::{Routed, Router};
pub mod server;
pub mod util;

// TODO actual error handling
#[derive(Debug, Copy, Clone, Default)]
#[derive(thiserror::Error)]
pub struct TempError;

impl std::fmt::Display for TempError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error!")
    }
}
