pub mod init;
use std::fmt;

pub use init::Init;

/// A unique* identifier for a game. The [Server](crate::Server) and all [Client](crate::Client)s will
/// share the same [Id].
/// *The id is persisted through [Save](crate::server::Save)s, so it is possible to 'fork' a
/// game and have two servers with the same game id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Id(uuid::Uuid);

impl Id {
    pub(crate) fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// A not-guaranteed-unique shortening of the Id for human-readable purposes only
    /// `format!("{:x}", game_id.friendly_display())``
    pub fn friendly_display(&self) -> impl std::fmt::LowerHex + std::fmt::UpperHex {
        self.0.as_u64_pair().0 as u32
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
