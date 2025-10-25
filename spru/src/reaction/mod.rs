use std::collections::VecDeque;

use derive_where::derive_where;
use telety::telety;

use crate::{action, interactor, item::lookup::Canonical, player};

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    pub root: &'r Root,
    pub player: Option<player::Id>,
}

impl<'r, Root> crate::interactor::GetRoot for Context<'r, Root> {
    type Root = Root;

    fn get_root(&self) -> &Self::Root {
        self.root
    }
}

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

#[telety(crate::reaction)]
pub trait Reaction {
    type State: tagset::TagSet;
    type Action;
    type Root;
    type Trigger;
    type GameOutcome;

    fn apply<'l, 'r>(
        &self,
        interactor: &mut Interactor<'l, 'r, Self>,
        trigger: Self::Trigger,
    ) -> action::Result<()>;
}

pub type Interactor<'l, 'r, Reaction> = crate::Interactor<
    'l,
    Canonical<
        <<Reaction as self::Reaction>::State as tagset::TagSet>::Repr,
        <Reaction as self::Reaction>::State
    >,
    <Reaction as self::Reaction>::Action,
    Context<'r, <Reaction as self::Reaction>::Root>,
    Output<<Reaction as self::Reaction>::Trigger, <Reaction as self::Reaction>::GameOutcome>,
>;
