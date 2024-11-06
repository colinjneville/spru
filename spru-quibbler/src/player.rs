use std::convert;

use spru::{error::LookupInteractionError, interaction::Interactor, item::IdT};
use spru_bevy::item::{self, lookup, BevyLookupMut};
use spru_util::{item::{counter, Counter, fsm, Fsm}, action::verbatim};
use rust_fsm::state_machine;

use crate::component::*;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub username: String,
    // ip...
}

impl Input {
    pub fn new(username: String) -> Self {
        Self {
            username,
        }
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Init;

impl spru::init::Base for Init {
    type In = Input;
    type Out = IdT<Root>;
    type Error = convert::Infallible;
    type ActionCatalog = crate::Actions;
}

impl<'l> spru::Init<IdT<Root>, BevyLookupMut<'l>> for Init 
where crate::Actions: spru::action::catalog::Apply<BevyLookupMut<'l>> {
    fn initialize(&self, interactor: &mut Interactor<BevyLookupMut<'l>, Self::ActionCatalog, IdT<Root>>, input: Self::In) -> Result<Self::Out, LookupInteractionError<lookup::BevyError, Self::Error>> {
        let score = interactor.create(counter::Create::default()).map_err(LookupInteractionError::Lookup)?;
        let hand = interactor.create(hand::Create::default()).map_err(LookupInteractionError::Lookup)?;
        let state = interactor.create(fsm::Create::default()).map_err(LookupInteractionError::Lookup)?;
        let root = interactor.create(verbatim::Create::new(Root {
            data: input,
            hand,
            score,
            state,
        })).map_err(LookupInteractionError::Lookup)?;

        Ok(root)
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub data: Input,
    pub hand: IdT<Hand>,
    pub score: IdT<Counter<u16>>,
    pub state: IdT<Fsm<state::Impl>>,
}

// #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
// #[derive(serde::Serialize, serde::Deserialize)]
// pub enum State {
//     Idle,
//     Draw,
//     Play,
// }

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub state(Idle)

    Idle(StartTurn) => Draw,
    Draw(Draw) => Play,
    Play => {
        Play => Idle,
        Pass => Idle,
    }
}

pub use state::Impl;

