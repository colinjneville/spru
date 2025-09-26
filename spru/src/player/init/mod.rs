pub mod error;
pub use error::Error;

use std::collections::VecDeque;

use crate::{interactor, item::{self}, player};

pub trait Init {
    type In;
    type Root;
    type State;
    type Action;

    fn initialize(&self, interactor: &mut Interactor<Self::State, Self::Action, Self::Root>, input: Self::In) 
        -> self::Result<()>;
}

pub type Interactor<'l, 'r, State, Action, Root> = crate::Interactor<'l, item::lookup::Canonical<State>, Action, Context<'r, Root>, Output>;

pub(crate) type Complete<'r, Action, Root> = crate::interactor::Complete<Action, self::Context<'r, Root>, Output>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    pub root: &'r Root,
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
        &self.root
    }
}

#[derive(Debug, Default)]
#[doc(hidden)]
pub struct Output {

}

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

pub type Result<T> = std::result::Result<T, self::Error>;
