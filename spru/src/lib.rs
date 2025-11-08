//! An experimental framework for building multiplayer strategy and digital board games.  
//!
//! spru is designed to structuralize game logic in such a way that synchronization, saving,
//! undo, simultaneous actions, hidden information, can be managed automatically. It is made
//! for portability and flexibility - it is WASM-compatible, not tied to any specific engine,
//! and works using user-defined Rust types (which you can use to layer a scripting language on top,
//! if desired).  
//! `spru-bevy` provides an implementation for the [bevy](https://bevyengine.org/) game engine.
//!
//! # Key engine types
//! These types are mainly needed for engine/networking integration. If you are building a game
//! on top of an existing integration such as `spru-bevy`, you may not need to interact with these
//! directly.
//!
//! - [Server]: The authoritative manager of the game state. Each server runs one game, and can have
//!   any number of clients (1 per player).  
//!   The server does no communication on its own, operations on the server will return signals a
//!   higher-layer must deliver to clients.
//! - [Client]: A player's interface to the game state. Each client belongs to a single player, but
//!   multiple clients can run in the same process (e.g. for hotseat games), and in the same process
//!   as the server. Each client has its own representation of the game state, which can be tentatively
//!   modified without affecting the server or other clients. The server will update clients with changes
//!   to the game state as needed.  
//!   A Client is created from the [Seed](common::Seed) returned after adding the player on the Server.  
//!   The client does no communication on its own, operations on the client will return signals a
//!   higher-layer must deliver to the server.
//! - [ToServer](common::signal::ToServer)/[ToClient](common::signal::ToClient): Abstracted communications
//!   between clients and the server.
//! - [Item]: A game object's [trait@State] along with additional metadata. Their storage is managed by the
//!   [Storage](item::Storage).
//! - [Storage](item::Storage): A manager for the storage of game items. A standalone implementation is
//!   provided in `spru-util`, but implementations can integrate with existing engine features,
//!   such as `spru-bevy`'s implmentation which creates the items as Bevy entities, which enables
//!   using bevy's event system, and querying game items in systems.
//!
//! # Key game traits
//! A game must implement these traits to define its logic:
//! - [trait@State]: A set of all types representing the state of mutable game objects. For example, a State
//!   could contain types representing the contents of a deck of cards, a player's life point counter,
//!   or something more abstract, like a list of all players in the game.
//! - [trait@Action]: A set of all types representing actions that create, update, or destroy game objects.
//!   For example, Action could contain types shuffling a deck, incrementing a counter, or adding a
//!   new player to the list of players.
//! - [trait@Interaction]: A player-initiated operation which modifies the [trait@State] of game objects by issuing [trait@Action]s.
//!   For example, an Interaction could be playing a card, moving a chess piece, or passing the turn.
//! - [trait@Reaction]: A server-side operation usually triggered by an [trait@Interaction] or another [trait@Reaction]. Reactions
//!   function similarly to Interactions, but being server-run, have more privileges. For example, a Reaction
//!   could be shuffling the discard pile into the deck once it is empty, or setting things up for a new round.
//!   Reactions are also able to determine when the game is over.
//! - [game::Init]: Initializes the game state, before any players are added.
//! - [player::Init]: Initializes a new player, and creates or updates any game objects necessary. This also can
//!   perform player validation, such as rejecting new players once the player limit has been reached.
//!
//! # Client Interfacing
//! Players interact with the game through their associated [Client] with [trait@Interaction]s. A player first
//! [stages](Client::stage_interaction) an Interaction, updating their local copy of the game stage with the
//! tentative changes. That interaction must be [applied](Client::apply_interactions) before it is sent to the [Server], which will approve or
//! reject the Interaction. Your interface can stage multiple Interactions before applying any of them, or immediately
//! apply after staging. The player can cancel staged Interactions by [reverting](Client::revert_interactions) them instead.  
//!
//! # tagset
//! [tagset] enables creating tagged unions which can easily implement traits, usually by delegating to their variant types.
//! This allows building reusable components which can be incorporated into any game.
//! [tagset] is currently required when implementing [trait@State], and recommended for [trait@Action]. See crate documentation for details.
//!
#![deny(missing_debug_implementations)]
#![allow(clippy::type_complexity)]
#![allow(
    clippy::crate_in_macro_def,
    reason = "'crate' in telety is already translated to this crate"
)]

pub mod action;
#[doc(inline)]
pub use action::Action;
pub mod client;
#[doc(inline)]
pub use client::Client;
pub mod common;
#[doc(inline)]
pub use common::Common;
pub mod state;
#[doc(inline)]
pub use state::State;
pub mod game;
pub mod interactor;
#[doc(inline)]
pub use interactor::Interactor;
pub mod item;
#[doc(inline)]
pub use item::Item;
pub mod interaction;
#[doc(inline)]
pub use interaction::Interaction;
pub mod player;
pub mod reaction;
#[doc(inline)]
pub use reaction::Reaction;
pub(crate) mod record;
pub mod server;
#[doc(inline)]
pub use server::Server;
pub(crate) mod transaction;
pub(crate) use transaction::Transaction;
mod visibility;

pub use tagset;

#[doc(hidden)]
pub mod __private {
    pub use serde;
    pub use telety;
}

mod sealed {
    #[doc(hidden)]
    pub trait Sealed {}
}
