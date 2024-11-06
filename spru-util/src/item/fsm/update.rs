use std::mem;

use crate::Strictness;

use super::*;

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "
    FSM::Input: Serial,
", serialize = "
    FSM::Input: Serial, 
"))]
#[spru::update(Undo = Set<FSM>, Error = error::Transition)]
pub struct Transition<FSM: StateMachineTy> {
    input: FSM::Input,
    strictness: Strictness,
}

impl<FSM: StateMachineTy> Transition<FSM> {
    pub fn new(input: FSM::Input) -> Self {
        Self {
            input,
            strictness: Strictness::AllOrError,
        }
    }

    pub fn new_best_effort(input: FSM::Input) -> Self {
        Self {
            input,
            strictness: Strictness::BestEffort,
        }
    }
}

impl<FSM> spru::Action for Transition<FSM>
where 
    FSM: StateMachineTy<
        State: Serial + Clone, 
        Input: Serial + Clone
    >,
{
    type T = Fsm<FSM>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        match FSM::transition(&input.0, &self.input) {
            Some(new_state) => {
                let old_state = mem::replace(&mut input.0, new_state);
                Ok(Some(Set::new(old_state)))
            }
            None => {
                match self.strictness {
                    Strictness::BestEffort => Ok(None),
                    Strictness::AllOrError => Err(error::Transition::TransitionImpossible(rust_fsm::TransitionImpossibleError)),
                }
            }
        }
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "
    FSM::State: Serial,
", serialize = "
    FSM::State: Serial, 
"))]
#[spru::update]
pub struct Set<FSM: StateMachineTy> {
    new_state: FSM::State,
}

impl<FSM: StateMachineTy> Set<FSM> {
    pub fn new(new_state: FSM::State) -> Self {
        Self {
            new_state,
        }
    }
}

impl<FSM> spru::Action for Set<FSM> 
where 
    FSM: StateMachineTy<State: Serial + Clone>,
{
    type T = Fsm<FSM>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        let old_state = mem::replace(&mut input.0, self.new_state.clone());
        Ok(Self::new(old_state))
    }
}