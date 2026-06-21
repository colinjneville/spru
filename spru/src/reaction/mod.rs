use std::collections::VecDeque;

use derive_where::derive_where;
use telety::telety;

use crate::{action, interactor, item::storage::Canonical, player};

/// Contextual information available through the [Interactor] in a [trait@Reaction].
#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    /// The root object of the game
    pub root: &'r Root,
    /// The player this [trait@Reaction] is in the context of.
    /// This will be `Some` for [trait@Reaction]s triggered by [Interaction](trait@crate::Interaction)s
    /// and [player::Init], and `None` for [Server::manual_trigger](crate::Server::manual_trigger).
    pub player: Option<player::Id>,
}

impl<'r, Root> crate::interactor::GetRoot for Context<'r, Root> {
    type Root = Root;

    fn get_root(&self) -> &Self::Root {
        self.root
    }
}

#[doc(hidden)]
#[derive(Debug)]
#[derive_where(Default)]
pub struct Output<Trigger, GameOutcome> {
    triggers: VecDeque<Trigger>,
    game_outcome: Option<GameOutcome>,
}

impl<Trigger, GameOutcome> interactor::EnqueueTrigger for Output<Trigger, GameOutcome> {
    type Trigger = Trigger;

    fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.triggers.push_back(trigger);
    }
}

impl<Trigger, GameOutcome> interactor::TakeTriggers<Trigger> for Output<Trigger, GameOutcome> {
    fn take_triggers(&mut self) -> VecDeque<Trigger> {
        std::mem::take(&mut self.triggers)
    }
}

impl<Trigger, GameOutcome> interactor::SetGameOutcome for Output<Trigger, GameOutcome> {
    type GameOutcome = GameOutcome;

    fn set_game_outcome(&mut self, game_outcome: Self::GameOutcome) {
        self.game_outcome = Some(game_outcome);
    }
}

impl<Trigger, GameOutcome> interactor::TakeGameOutcome<GameOutcome>
    for Output<Trigger, GameOutcome>
{
    fn take_game_outcome(&mut self) -> Option<GameOutcome> {
        self.game_outcome.take()
    }
}

/// Server-side game logic initiated by a [Reaction::Trigger].  
/// 
/// After an [Interaction](trait@crate::Interaction) or [trait@Reaction] completes, the next (FIFO) Trigger enqueued
/// with [Interactor::enqueue_trigger](crate::Interactor::enqueue_trigger) starts a new Reaction. This continues until
/// no Triggers remain.
/// A Reaction can also set the [Reaction::GameOutcome]. If an endgame condition has been met,
/// (e.g. a king is in checkmate, the final round has completed, etc.), call
/// [Interactor::set_game_outcome] to record the winner and any other desired information.
/// Clients will be notified of the `GameOutcome` along with this transaction.
/// Reactions should avoid failing when possible. Validation should be done in [Interaction](trait@crate::Interaction)s
/// so players can be notified of the failure locally as soon as possible.
#[telety(crate::reaction)]
pub trait Reaction {
    /// The game's [trait@crate::Action]
    type Action: crate::Action;
    /// The game's [Common::Root](crate::Common::Root)
    type Root;
    /// The input that determines what the reaction should do
    type Trigger;
    /// The game's [Common::GameOutcome](crate::Common::GameOutcome)
    type GameOutcome;

    /// Perform the reaction for the given [Reaction::Trigger].
    /// This is similar to [Interaction::apply](crate::Interaction::apply), but it has access
    /// to all game [Item](crate::Item)s because it is only run on the [Server](crate::Server).
    /// Additionally, it can call [Interactor::set_game_outcome] to end the game.
    fn apply<'l, 'r>(
        &self,
        interactor: &mut Interactor<'l, 'r, Self>,
        trigger: Self::Trigger,
    ) -> action::Result<()>;
}

/// An alias for the [Interactor](crate::Interactor) used in a [trait@Reaction](trait@Reaction).
pub type Interactor<'l, 'r, Reaction> = crate::Interactor<
    'l,
    Canonical<
        <<<Reaction as self::Reaction>::Action as crate::Action>::State as tagset::TagSet>::Repr,
        <<Reaction as self::Reaction>::Action as crate::Action>::State,
    >,
    <Reaction as self::Reaction>::Action,
    Context<'r, <Reaction as self::Reaction>::Root>,
    Output<<Reaction as self::Reaction>::Trigger, <Reaction as self::Reaction>::GameOutcome>,
>;
