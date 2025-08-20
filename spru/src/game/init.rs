use crate::item;

pub trait Init {
    type Root;
    type Error;
    type State: crate::State<item::lookup::Canonical<Self::State>>;
    type Action: crate::Action<item::lookup::Canonical<Self::State>>;

    fn initialize(self, interactor: &mut Interactor<Self::State, Self::Action>) 
        -> Result<Self::Root, Error<Self::Error>>;
}

pub type Interactor<'l, State, Action> = crate::Interactor<'l, item::lookup::Canonical<State>, Action, Context>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Context {
    
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<InitError> {
    Lookup(#[from] item::lookup::canonical::Error),
    #[error(transparent)]
    Init(InitError),
}