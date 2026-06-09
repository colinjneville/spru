pub mod error;
pub use error::Error;

use std::collections::VecDeque;

use crate::{
    interactor,
    item::{self},
};

/// Initializes a game.  
/// This is used once during [Server::init](crate::Server::init) to create the [Root](Init::Root)
/// and any other initial state.
pub trait Init {
    /// The game's [Common::Root](crate::Common::Root).
    /// Created by [Init::initialize].
    type Root;
    /// The game's [trait@crate::Action]
    type Action: crate::Action;

    /// Initialize the game and return the [Root](Init::Root).
    fn initialize(self, interactor: &mut Interactor<Self>) -> self::Result<Self::Root>;
}

/// An alias for the [Interactor](crate::Interactor) used in [game::Init](Init).
pub type Interactor<'l, Init> = crate::Interactor<
    'l,
    item::storage::Canonical<
        <<<Init as self::Init>::Action as crate::Action>::State as tagset::TagSet>::Repr,
        <<Init as self::Init>::Action as crate::Action>::State,
    >,
    <Init as self::Init>::Action,
    Context,
    Output,
>;

/// [Interactor] context object. Contains no context, any required data should be contained
/// within the [Init] object.
#[derive(Debug)]
#[non_exhaustive]
pub struct Context {}

impl interactor::PlayerContext for Context {
    fn player_context(&self) -> Option<crate::player::Id> {
        None
    }
}

#[derive(Debug, Default)]
#[doc(hidden)]
pub struct Output {}

impl<Trigger> interactor::TakeTriggers<Trigger> for Output {
    fn take_triggers(&mut self) -> VecDeque<Trigger> {
        VecDeque::new()
    }
}

impl<GameOutcome> interactor::TakeGameOutcome<GameOutcome> for Output {
    fn take_game_outcome(&mut self) -> Option<GameOutcome> {
        None
    }
}

/// A result with an [Error] `Err`
pub type Result<T> = std::result::Result<T, self::Error>;
