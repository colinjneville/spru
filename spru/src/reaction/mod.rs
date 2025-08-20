use std::collections::VecDeque;

use telety::telety;

use crate::{item::lookup::{canonical, Canonical}, player};

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root, Trigger, GameOutcome> {
    pub root: &'r Root,
    pub player: Option<player::Id>,
    triggers: VecDeque<Trigger>,
    game_outcome: Option<GameOutcome>,
}

impl<'r, Root, Trigger, GameOutcome> Context<'r, Root, Trigger, GameOutcome> {
    pub(crate) fn new(root: &'r Root, player: Option<player::Id>, triggers: VecDeque<Trigger>) -> Self {
        Self {
            root,
            player,
            triggers,
            game_outcome: None,
        }
    }

    pub(crate) fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.triggers.push_back(trigger);
    }

    pub(crate) fn dequeue_trigger(&mut self) -> Option<Trigger> {
        self.triggers.pop_front()
    }

    pub(crate) fn set_game_outcome(&mut self, game_outcome: GameOutcome) -> Option<GameOutcome> {
        self.game_outcome.replace(game_outcome)
    }

    pub(crate) fn take_game_outcome(&mut self) -> Option<GameOutcome> {
        self.game_outcome.take()
    }
}

#[telety(crate::reaction)]
pub trait Reaction {
    type State;
    type Action: crate::Action<Canonical<Self::State>>;
    type Root;
    type Trigger;
    type GameOutcome;
    
    fn apply<'l, 'r>(
        &self, 
        interactor: &mut Interactor<'l, 'r, Self::State, Self::Action, Self::Root, Self::Trigger, Self::GameOutcome>, 
        trigger: Self::Trigger
    ) 
        -> Result<(), Error>;
}

pub type Interactor<'l, 'r, State, Action, Root, Trigger, GameOutcome> = crate::Interactor<'l, Canonical<State>, Action, Context<'r, Root, Trigger, GameOutcome>>;

pub type Error = canonical::Error;

// #[derive(Debug)]
// #[derive(thiserror::Error)]
// pub enum Error {
//     #[error(transparent)]
//     Lookup(canonical::Error),
// }