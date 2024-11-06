pub mod action;
pub use action::Action;
pub mod client;
pub use client::Client;
pub mod error;
mod history;
pub use history::History;
pub mod init;
pub use init::Init;
pub mod item;
pub use item::Item;
pub mod interaction;
pub use interaction::Interaction;
pub mod log;
pub mod player;
pub use player::Player; 
pub mod record;
pub use record::Record;
pub mod save;
pub use save::Save;
pub mod server;
pub use server::Server;
pub mod snapshot;
pub use snapshot::Snapshot;
pub mod transaction;
pub use transaction::Transaction;
mod visibility;
pub use visibility::Visibility;
mod zone;
pub use zone::Zone;

pub use spru_macro::{FromInfallible, create, destroy, update};

#[doc(hidden)]
pub mod __private {
    pub use telety;
    pub use amass;

    #[path = "../type_index.rs"]
    pub mod type_index;

    pub use crate::item::catalog::do_apply_item;
}

// TODO this is gross
pub(crate) use __private::type_index;

pub trait Serial: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static { }

impl<T: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static> Serial for T { }

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Player {player} has desynced due to implementation error: {message}")]
pub struct SyncError {
    player: player::Id,
    message: String,
}

impl SyncError {
    pub fn new<S: Into<String>>(player: player::Id, message: S) -> Self {
        let message = message.into();
        Self {
            player,
            message,
        }
    }
}

// TODO actual errors
#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Error!")]
pub struct TempError;

impl TempError {
    pub fn discard<T>(_t: T) -> Self {
        Self
    }
}
