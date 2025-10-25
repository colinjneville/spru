pub mod error;
pub use error::Error;

use std::{collections::VecDeque, fmt};

use derive_where::derive_where;
use tagset::tagset_meta;
use telety::telety;

use crate::{interactor, item, player};

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    pub root: &'r Root,
    pub player: player::Id,
}

impl<'r, Root> interactor::PlayerContext for Context<'r, Root> {
    fn player_context(&self) -> Option<player::Id> {
        Some(self.player)
    }
}

impl<'r, Root> crate::interactor::GetRoot for Context<'r, Root> {
    type Root = Root;

    fn get_root(&self) -> &Self::Root {
        self.root
    }
}

#[derive(Debug)]
#[derive_where(Default)]
#[doc(hidden)]
pub struct Output<Trigger> {
    pub(crate) triggers: VecDeque<Trigger>,
}

impl<Trigger> interactor::EnqueueTrigger for Output<Trigger> {
    type Trigger = Trigger;

    fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.triggers.push_back(trigger);
    }
}

impl<Trigger> interactor::TakeTriggers<Trigger> for Output<Trigger> {
    fn take_triggers(&mut self) -> VecDeque<Trigger> {
        std::mem::take(&mut self.triggers)
    }
}

impl<Trigger, GameOutcome> interactor::TakeGameOutcome<GameOutcome> for Output<Trigger> {
    fn take_game_outcome(&mut self) -> Option<GameOutcome> {
        None
    }
}

impl<'r, Root> Context<'r, Root> {
    pub(crate) fn new(root: &'r Root, player: player::Id) -> Self {
        Self { root, player }
    }
}

#[telety(crate::interaction, alias_traits = "always")]
#[tagset_meta]
pub trait Interaction {
    type State: crate::State;
    type Action: crate::Action<State = Self::State>;
    type Root;
    type Trigger;

    fn apply<'l, 'r, Lookup>(
        &self,
        interactor: &mut Interactor<'l, 'r, Lookup, Self>,
    ) -> self::Result<()>
    where
        Lookup: item::Lookup<State = Self::State>;
}

pub type Interactor<'l, 'r, Lookup, Interaction> =
    crate::Interactor<'l,
        Lookup,
        <Interaction as self::Interaction>::Action,
        Context<'r,
            <Interaction as self::Interaction>::Root>,
            Output<<Interaction as self::Interaction>::Trigger
        >
    >;

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Staged<Interaction> {
    pub(crate) interaction: Interaction,
    pub(crate) expected_versions: item::version::Expected,
    pub(crate) pending_interaction_id: Pending,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
pub struct Pending(u32);

impl Pending {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "i{}", self.0)
    }
}
