pub mod error;
mod seed;
pub use seed::Seed;
pub mod signal;
mod snapshot;
pub(crate) use snapshot::Snapshot;

use std::marker::PhantomData;

use derive_where::derive_where;

/// Associated types shared between [Server](crate::Server) and [Client](crate::Client).  
/// [Client::Common](crate::Client::Common) == [Server::Common](crate::Server::Common) for clients and servers in the same game.  
#[allow(missing_docs)]
pub trait Common: crate::sealed::Sealed + Sized {
    /// The set of all [Item](crate::Item) types in the game.  
    /// See [State](trait@crate::State).
    type State;
    /// The set of all actions that can be applied to [State](Common::State)s.  
    /// See [Action](trait@crate::Action).
    type Action;
    /// The type of a value accessible to all [Interactor](crate::Interactor)s.  
    /// All game items should be accessible from this value.
    type Root;
    /// Player-initiated interaction with the game state.  
    /// See [Interaction](trait@crate::Interaction).
    type Interaction;
    /// The final outcome of the game.  
    type GameOutcome;
}

#[doc(hidden)]
pub type CommonImpl<Interaction, GameOutcome> = Impl<
    <Interaction as crate::Interaction>::State,
    <Interaction as crate::Interaction>::Action,
    <Interaction as crate::Interaction>::Root,
    Interaction,
    GameOutcome,
>;

#[doc(hidden)]
#[derive_where(Debug)]
pub struct Impl<State, Action, Root, Interaction, GameOutcome> {
    _p: PhantomData<fn() -> (State, Action, Root, Interaction, GameOutcome)>,
}

impl<State, Action, Root, Interaction, GameOutcome> crate::sealed::Sealed
    for Impl<State, Action, Root, Interaction, GameOutcome>
{
}

impl<State, Action, Root, Interaction, GameOutcome> Common
    for Impl<State, Action, Root, Interaction, GameOutcome>
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type Interaction = Interaction;
    type GameOutcome = GameOutcome;
}

pub(crate) type SeqId = i32;
