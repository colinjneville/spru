pub mod error;

use std::mem;

use derive_where::derive_where;
pub use rust_fsm;
use spru::common::error::AnyResult;
use spru_script::script;
use tagset::tagset;
use telety::telety;

use crate::{Strictness, cloned};

/// A type which can be used in [Fsm].
pub trait StateMachineTy: Clone + rust_fsm::StateMachineImpl + 'static {}

impl<T: Clone + rust_fsm::StateMachineImpl + 'static> StateMachineTy for T {}

/// A finite-state machine, useful for tracking things like turn phases.
/// Powered by [rust_fsm]'s state machines.
#[derive_where(Debug, Clone, Serialize, Deserialize; FSM::State)]
#[script(include = [Methods])]
pub struct Fsm<FSM: StateMachineTy>(FSM::State);

#[script(partial = Methods)]
impl<FSM> Fsm<FSM> 
where
    FSM: StateMachineTy,
    FSM::State: Clone,
{
    #[create]
    fn create(initial_state: FSM::State) -> Create<FSM> {
        create(initial_state)
    }

    #[create]
    fn dflt() -> Create<FSM> {
        default()
    }

    #[method]
    fn destroy(&self) -> ((), cloned::Destroy<Fsm<FSM>>) {
        ((), destroy())
    }

    #[get(name = current)]
    fn _current(&self) -> FSM::State {
        self.current().clone()
    }

    #[method]
    fn transition(&self, input: FSM::Input) -> (Option<FSM::Output>, Transition<FSM>) {
        let output = <FSM as rust_fsm::StateMachineImpl>::output(&self.0, &input);
        (output, transition(input))
    }

    #[method]
    fn try_transition(&self, input: FSM::Input) -> (Option<FSM::Output>, Transition<FSM>) {
        let output = <FSM as rust_fsm::StateMachineImpl>::output(&self.0, &input);
        (output, try_transition(input))
    }

    #[method]
    fn set(&self, new_state: FSM::State) -> ((), Set<FSM>) {
        ((), set(new_state))
    }
}

impl<FSM: StateMachineTy> Fsm<FSM> {
    pub fn current(&self) -> &FSM::State {
        &self.0
    }
}

pub fn default<FSM: StateMachineTy>() -> Create<FSM> {
    create(FSM::INITIAL_STATE)
}

pub fn create<FSM: StateMachineTy>(initial_state: FSM::State) -> Create<FSM> {
    cloned::create(Fsm(initial_state))
}

/// Transition using the provided input. Fails if the input is not
/// permitted for the current state.
pub fn transition<FSM: StateMachineTy>(input: FSM::Input) -> Transition<FSM> {
    Transition {
        input,
        strictness: Strictness::AllOrError,
    }
}

/// Attempt a transition, but ignore any error
pub fn try_transition<FSM: StateMachineTy>(input: FSM::Input) -> Transition<FSM> {
    Transition {
        input,
        strictness: Strictness::BestEffort,
    }
}

/// Switch directly to the given state
pub fn set<FSM: StateMachineTy>(new_state: FSM::State) -> Set<FSM> {
    Set { new_state }
}

pub fn destroy<FSM: StateMachineTy>() -> Destroy<FSM> {
    cloned::destroy()
}

#[telety(crate::fsm)]
#[tagset(Create<FSM>)]
#[tagset(Set<FSM>)]
#[tagset(Transition<FSM>)]
#[tagset(Destroy<FSM>)]
#[tagset(reserved(..8))]
pub struct Actions<FSM: StateMachineTy>;

#[derive(Debug, Clone, crate::FromInfallible, thiserror::Error)]
#[error("FSM error: {0}")]
pub enum Error {
    Transition(#[from] error::Transition),
}

pub type Create<FSM> = cloned::Create<Fsm<FSM>>;

pub type Destroy<FSM> = cloned::Destroy<Fsm<FSM>>;

#[derive_where(Debug, Clone, Serialize, Deserialize; FSM::Input)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Transition<FSM: StateMachineTy> {
    input: FSM::Input,
    strictness: Strictness,
}

impl<FSM> spru::action::Update for Transition<FSM>
where
    FSM: StateMachineTy<State: Clone, Input: Clone>,
{
    type T = Fsm<FSM>;
    type Undo = Set<FSM>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        match FSM::transition(&value.0, &self.input) {
            Some(new_state) => {
                let old_state = mem::replace(&mut value.0, new_state);
                Ok(Some(Set {
                    new_state: old_state,
                }))
            }
            None => match self.strictness {
                Strictness::BestEffort => Ok(None),
                Strictness::AllOrError => Err(error::Transition::TransitionImpossible(
                    rust_fsm::TransitionImpossibleError,
                )
                .into()),
            },
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
    FSM: StateMachineTy<State: Clone>,
{
    type T = Fsm<FSM>;
    type Undo = Self;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let old_state = mem::replace(&mut value.0, self.new_state.clone());
        Ok(Self {
            new_state: old_state,
        })
    }
}
