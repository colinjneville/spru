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
    type State: crate::State;
    /// The set of all actions that can be applied to [State](Common::State)s.  
    /// See [Action](trait@crate::Action).
    type Action: crate::Action<State = Self::State>;
    /// The type of a value accessible to all [Interactor](crate::Interactor)s.  
    /// All game items should be accessible from this value.
    type Root;
    /// Player-initiated interaction with the game state.  
    /// See [Interaction](trait@crate::Interaction).
    type Interaction: crate::Interaction<Action = Self::Action, Root = Self::Root>;
    /// The final outcome of the game.  
    type GameOutcome;
}

#[doc(hidden)]
pub type CommonImpl<Interaction, GameOutcome> = Impl<
    Interaction,
    GameOutcome,
>;

#[doc(hidden)]
#[derive_where(Debug)]
pub struct Impl<Interaction, GameOutcome> {
    _p: PhantomData<(Interaction, GameOutcome)>,
}

impl<Interaction, GameOutcome> crate::sealed::Sealed
    for Impl<Interaction, GameOutcome>
{
}

impl<Interaction, GameOutcome> Common
    for Impl<Interaction, GameOutcome>
where 
    Interaction: crate::Interaction,
{
    type State = <Interaction::Action as crate::Action>::State;
    type Action = Interaction::Action;
    type Root = Interaction::Root;
    type Interaction = Interaction;
    type GameOutcome = GameOutcome;
}

pub(crate) type SeqId = i32;
