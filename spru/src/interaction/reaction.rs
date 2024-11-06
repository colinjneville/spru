use crate::{action, interaction::Interactor, item::{lookup::{canonical, Canonical}}};

pub trait Reaction<ItemCatalog, ActionCatalog, Root> {
    type Input;
    type GameOutcome;
    
    fn apply(&self, interactor: &mut Interactor<Canonical<ItemCatalog>, ActionCatalog, Root>, input: Self::Input) -> Result<Option<Self::GameOutcome>, Error>
    where 
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>>,
    ;
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Lookup(canonical::Error),
}