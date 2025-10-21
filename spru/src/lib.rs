#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]

pub mod action;
pub use action::Action;
pub mod client;
pub use client::Client;
mod common;
pub use common::Common;
pub mod state;
pub use state::State;
pub mod error;
pub use error::{AnyError, TempError, TempResult, PsuedoError};
pub mod game;
pub mod interactor;
pub use interactor::Interactor;
pub mod item;
pub use item::Item;
pub mod interaction;
pub use interaction::Interaction;
pub mod log;
pub mod player;
pub mod reaction;
pub use reaction::Reaction;
pub mod record;
pub use record::Record;
pub mod server;
pub use server::Server;
pub mod transaction;
pub use transaction::Transaction;
mod visibility;
pub use visibility::Visibility;

pub use spru_macro::FromInfallible;

pub trait Serial: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static { }

impl<T: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static> Serial for T { }

#[doc(hidden)]
pub mod __private {
    pub use telety;
    pub use serde;
}
