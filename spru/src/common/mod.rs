use std::marker::PhantomData;

use derive_where::derive_where;

pub trait Common: Sized {
    type State;
    type Action;
    type Root;
    type GameOutcome;
    type Interaction;
}

#[derive_where(Debug)]
pub struct Impl<State, Action, Root, GameOutcome, Interaction> {
    _p: PhantomData<fn() -> (State, Action, Root, GameOutcome, Interaction)>,
}

impl<State, Action, Root, GameOutcome, Interaction> Common for Impl<State, Action, Root, GameOutcome, Interaction> {
    type State = State;
    type Action = Action;
    type Root = Root;
    type GameOutcome = GameOutcome;
    type Interaction = Interaction;
}