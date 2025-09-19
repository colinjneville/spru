pub mod error;

use std::mem;

use amass::amass_telety;
use derive_where::derive_where;
use spru::{error::AnyResult, Serial};
pub use rust_fsm::StateMachineImpl;
use tagset::tagset;
use telety::telety;

use crate::{verbatim, Strictness};

pub trait StateMachineTy: Clone + StateMachineImpl + 'static { }

impl<T: Clone + StateMachineImpl + 'static> StateMachineTy for T { }

#[derive_where(Debug, Clone, Serialize, Deserialize; FSM::State)]
pub struct State<FSM: StateMachineTy>(FSM::State);

impl<FSM: StateMachineTy> State<FSM> {
    
}

pub fn default<FSM: StateMachineTy>() -> Create<FSM> {
    create(FSM::INITIAL_STATE)
}

pub fn create<FSM: StateMachineTy>(initial_state: FSM::State) -> Create<FSM> {
    verbatim::create(State(initial_state))
}

pub fn transition<FSM: StateMachineTy>(input: FSM::Input) -> Transition<FSM> {
    Transition {
        input,
        strictness: Strictness::AllOrError,
    }
}

pub fn try_transition<FSM: StateMachineTy>(input: FSM::Input) -> Transition<FSM> {
    Transition {
        input,
        strictness: Strictness::BestEffort,
    }
}

pub fn set<FSM: StateMachineTy>(new_state: FSM::State) -> Set<FSM> {
    Set { new_state }
}

pub fn destroy<FSM: StateMachineTy>() -> Destroy<FSM> {
    verbatim::destroy()
}

#[telety(crate::fsm)]
#[tagset(Create<FSM>)]
#[tagset(Set<FSM>)]
#[tagset(Transition<FSM>)]
#[tagset(Destroy<FSM>)]
#[tagset(reserved(..8))]
pub struct Actions<FSM: StateMachineTy>;

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::fsm)]
pub enum Error {
    Transition(error::Transition),
}

pub type Create<FSM> = verbatim::Create<State<FSM>>;

pub type Destroy<FSM> = verbatim::Destroy<State<FSM>>;

#[derive_where(Debug, Clone, Serialize, Deserialize; FSM::Input)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Transition<FSM: StateMachineTy> {
    input: FSM::Input,
    strictness: Strictness,
}

impl<FSM> spru::action::Update for Transition<FSM>
where 
    FSM: StateMachineTy<
        State: Serial + Clone, 
        Input: Serial + Clone
    >,
{    
    type T = State<FSM>;
    type Undo = Set<FSM>;
    
    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        match FSM::transition(&value.0, &self.input) {
            Some(new_state) => {
                let old_state = mem::replace(&mut value.0, new_state);
                Ok(Some(Set { new_state: old_state }))
            }
            None => {
                match self.strictness {
                    Strictness::BestEffort => Ok(None),
                    Strictness::AllOrError => Err(error::Transition::TransitionImpossible(rust_fsm::TransitionImpossibleError).into()),
                }
            }
        }
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize; FSM::State)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Set<FSM: StateMachineTy> {
    new_state: FSM::State,
}

impl<FSM> spru::action::Update for Set<FSM> 
where 
    FSM: StateMachineTy<State: Serial + Clone>,
{
    type T = State<FSM>;
    type Undo = Self;
    
    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let old_state = mem::replace(&mut value.0, self.new_state.clone());
        Ok(Self { new_state: old_state })
    }
}