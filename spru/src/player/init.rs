use crate::{item, player};

pub trait Init {
    type In;
    type Root;
    type Error;
    type State: crate::State<item::lookup::Canonical<Self::State>>;
    type Action: crate::Action<item::lookup::Canonical<Self::State>>;

    fn initialize(&self, interactor: &mut Interactor<Self::State, Self::Action, Self::Root>, input: Self::In) 
        -> Result<(), Error<Self::Error>>;
}

pub type Interactor<'l, 'r, State, Action, Root> = crate::Interactor<'l, item::lookup::Canonical<State>, Action, Context<'r, Root>>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    pub root: &'r Root,
    pub player: player::Id,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<InitError> {
    Lookup(#[from] item::lookup::canonical::Error),
    #[error(transparent)]
    Init(InitError),
}