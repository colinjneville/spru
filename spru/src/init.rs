
use crate::{action, interaction::Interactor, item::{self}};

pub trait Base: Sized + 'static {
    type In;
    type Out;
    type Error;
}

pub trait Init<ItemCatalog, ActionCatalog, Root>: Base
where 
    ActionCatalog: action::Catalog<item::lookup::Canonical<ItemCatalog>>
{
    fn initialize(&self, interactor: &mut Interactor<item::lookup::Canonical<ItemCatalog>, ActionCatalog, Root>, input: Self::In) -> Result<Self::Out, Error<Self::Error>>;
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<InitError> {
    Lookup(#[from] item::lookup::canonical::Error),
    #[error(transparent)]
    Init(InitError),
}