#![deny(missing_debug_implementations)]
#![allow(clippy::type_complexity)]
#![allow(
    clippy::crate_in_macro_def,
    reason = "'crate' in telety is already translated to this crate"
)]
#![warn(missing_docs)]
//! spru

pub mod action;
pub use action::Action;
pub mod client;
pub use client::Client;
pub mod common;
pub use common::Common;
pub mod state;
pub use state::State;
pub mod game;
pub mod interactor;
pub use interactor::Interactor;
pub mod item;
pub use item::Item;
pub mod interaction;
pub use interaction::Interaction;
pub mod player;
pub mod reaction;
pub use reaction::Reaction;
pub(crate) mod record;
pub(crate) use record::Record;
pub mod server;
pub use server::Server;
pub(crate) mod transaction;
pub(crate) use transaction::Transaction;
mod visibility;
pub use visibility::Visibility;

#[doc(hidden)]
pub mod __private {
    pub use serde;
    pub use telety;
}
