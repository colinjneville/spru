use std::collections::VecDeque;

use tagset::tagset_meta;
use telety::telety;

use crate::{item, player};

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root, Trigger> {
    pub root: &'r Root,
    pub player: player::Id,
    triggers: VecDeque<Trigger>,
}

impl<'r, Root, Trigger> Context<'r, Root, Trigger> {
    pub(crate) fn new(root: &'r Root, player: player::Id) -> Self {
        Self {
            root,
            player,
            triggers: VecDeque::new(),
        }
    }

    pub(crate) fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.triggers.push_back(trigger);
    }

    pub(crate) fn into_triggers(self) -> VecDeque<Trigger> {
        self.triggers
    }
}


#[telety(crate::interaction, alias_traits = "always")]
#[tagset_meta]
pub trait Interaction {
    type Action;
    type Root;
    type Trigger;
    #[meta(default(std::convert::Infallible))]
    type Error;

    fn apply<'l, 'r, Lookup>(&self, interactor: &mut Interactor<'l, 'r, Lookup, Self::Action, Self::Root, Self::Trigger>)
         -> Result<(), Error<Lookup::Error, Self::Error>>
    where 
        Lookup: item::Lookup,
        Self::Action: crate::Action<Lookup>,
    ;
}

pub type Interactor<'l, 'r, Lookup, Action, Root, Trigger> = crate::Interactor<'l, Lookup, Action, Context<'r, Root, Trigger>>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, InteractionError> {
    Lookup(#[from] LookupError),
    Interaction(InteractionError),
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Staged<Interaction> {
    pub(crate) interaction: Interaction,
    pub(crate) expected_versions: item::version::Expected,
}
