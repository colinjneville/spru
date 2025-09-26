pub mod error;
pub use error::Error;

use std::collections::VecDeque;

use crate::{interactor, item::{self}};

pub trait Init {
    type Root;
    type State;
    type Action;

    fn initialize(self, interactor: &mut Interactor<Self::State, Self::Action>) 
        -> self::Result<Self::Root>;
}

pub type Interactor<'l, State, Action> = crate::Interactor<'l, item::lookup::Canonical<State>, Action, Context, Output>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Context {
    
}

impl interactor::PlayerContext for Context {
    fn player_context(&self) -> Option<crate::player::Id> {
        None
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