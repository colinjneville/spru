pub mod error;
pub use error::Error;

use std::collections::VecDeque;

use crate::{
    interactor,
    item::{self},
    player,
};

pub trait Init {
    /// The input required to initialize a player.  
    /// This parameter is provided by the caller of [Server::add_player](crate::Server::add_player).
    type In;
    /// The game's [Server::Root](crate::Server::Root)
    type Root;
    /// The game's [Server::State](crate::Server::State)
    type State: tagset::TagSet;
    /// The game's [Server::Action](crate::Server::Action)
    type Action;

    fn initialize(&self, interactor: &mut Interactor<Self>, input: Self::In) -> self::Result<()>;
}

/// An alias for the [Interactor](crate::Interactor) used in [player::Init].
pub type Interactor<'l, 'r, Init> = crate::Interactor<
    'l,
    item::storage::Canonical<
        <<Init as self::Init>::State as tagset::TagSet>::Repr,
        <Init as self::Init>::State,
    >,
    <Init as self::Init>::Action,
    Context<'r, <Init as self::Init>::Root>,
    Output,
>;

pub(crate) type Complete<'r, Action, Root> =
    crate::interactor::Complete<Action, self::Context<'r, Root>, Output>;

/// Additional context available during [player::Init].
#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    /// The game [Root](crate::Common::Root)
    pub root: &'r Root,
    /// The tentative id of the player being added
    pub player: player::Id,
}

impl<'r, Root> crate::interactor::PlayerContext for Context<'r, Root> {
    fn player_context(&self) -> Option<player::Id> {
        Some(self.player)
    }
}

impl<'r, Root, Trigger> interactor::TakeTriggers<Trigger> for Context<'r, Root> {
    fn take_triggers(&mut self) -> VecDeque<Trigger> {
        VecDeque::new()
    }
}

impl<'r, Root> crate::interactor::GetRoot for Context<'r, Root> {
    type Root = Root;

    fn get_root(&self) -> &Self::Root {
        self.root
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
