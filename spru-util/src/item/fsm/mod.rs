pub mod error;
pub mod update;

use std::{marker::PhantomData, fmt};

use amass::amass_telety;
use perfect_derive::perfect_derive;
use spru::Serial;
use rust_fsm::StateMachineImpl;

pub trait StateMachineTy: Clone + StateMachineImpl + 'static { }

impl<T: Clone + StateMachineImpl + 'static> StateMachineTy for T { }

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "
    FSM::State: Serial,    
", serialize = "
    FSM::State: Serial, 
"))]
pub struct Fsm<FSM: StateMachineTy>(FSM::State);

impl<FSM: StateMachineTy<State: Clone>> Clone for Fsm<FSM> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<FSM: StateMachineTy<State: fmt::Debug>> fmt::Debug for Fsm<FSM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Fsm").field(&self.0).finish()
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "
    FSM::Input: Serial,
    FSM::State: Serial,    
", serialize = "
    FSM::Input: Serial,
    FSM::State: Serial, 
"))]
#[derive(spru::action::Catalog)]
#[catalog(error = Error)]
#[amass_telety(crate::item::fsm)]
pub enum Catalog<FSM: StateMachineTy> {
    Create(Create<FSM>),
    Set(update::Set<FSM>),
    Transition(update::Transition<FSM>),
    Destroy(Destroy<FSM>),
}

// impl<FSM: StateMachineTy<Input: fmt::Debug, State: fmt::Debug>> fmt::Debug for Catalog<FSM> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Self::Create(arg0) => f.debug_tuple("Create").field(arg0).finish(),
//             Self::Set(arg0) => f.debug_tuple("Set").field(arg0).finish(),
//             Self::Transition(arg0) => f.debug_tuple("Transition").field(arg0).finish(),
//             Self::Destroy(arg0) => f.debug_tuple("Destroy").field(arg0).finish(),
//         }
//     }
// }

// impl<FSM: StateMachineTy<Input: Clone, State: Clone>> Clone for Catalog<FSM> {
//     fn clone(&self) -> Self {
//         match self {
//             Self::Create(arg0) => Self::Create(arg0.clone()),
//             Self::Set(arg0) => Self::Set(arg0.clone()),
//             Self::Transition(arg0) => Self::Transition(arg0.clone()),
//             Self::Destroy(arg0) => Self::Destroy(arg0.clone()),
//         }
//     }
// }

#[perfect_derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::item::fsm)]
pub enum Error {
    Transition(error::Transition),
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "
    FSM::State: Serial,    
", serialize = "
    FSM::State: Serial, 
"))]
#[spru::create(Undo = Destroy<FSM>)]
pub struct Create<FSM: StateMachineTy> {
    initial_state: FSM::State,
}

impl<FSM: StateMachineTy<State: fmt::Debug>> fmt::Debug for Create<FSM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Create").field("initial_state", &self.initial_state).finish()
    }
}

impl<FSM: StateMachineTy<State: Clone>> Clone for Create<FSM> {
    fn clone(&self) -> Self {
        Self { initial_state: self.initial_state.clone() }
    }
}

impl<FSM: StateMachineTy> Create<FSM> {
    pub fn new() -> Self {
        Self::new_with_state(FSM::INITIAL_STATE)
    }

    pub fn new_with_state(initial_state: FSM::State) -> Self {
        Self {
            initial_state,
        }
    }
}

impl<FSM: StateMachineTy> Default for Create<FSM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<FSM> spru::Action for Create<FSM> 
where 
    FSM: StateMachineTy<State: Serial + Clone>,
{
    type T = Fsm<FSM>;
    
    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok((Destroy::new(), Fsm(self.initial_state.clone())))
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::destroy(Undo = Create<FSM>)]
pub struct Destroy<FSM: StateMachineTy> 
where FSM::State: 'static {
    _p: PhantomData<fn() -> FSM>,
}

impl<FSM: StateMachineTy> Destroy<FSM> 
where FSM::State: 'static {
    pub fn new() -> Self {
        Self {
            _p: PhantomData,
        }
    }
}

impl<FSM> spru::Action for Destroy<FSM> 
where 
    FSM: StateMachineTy<State: 'static>,
{
    type T = Fsm<FSM>;
    
    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(Create::new_with_state(input.0))
    }
}
