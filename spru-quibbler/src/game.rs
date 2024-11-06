use std::convert;

use spru_bevy::{error::LookupInteractionError, item::{self, lookup, BevyLookupMut}};
use spru_util::{item::*, action::verbatim};

use crate::{data::{card, Card}, player};

pub struct Input;

pub struct Init;

impl spru::init::Base for Init {
    type In = Input;
    type Out = item::IdT<Root>;
    type Error = convert::Infallible;
}

impl<'l> spru::Init<item::IdT<player::Root>, BevyLookupMut<'l>> for Init
where crate::Actions: spru::action::catalog::Apply<BevyLookupMut<'l>> {
    fn initialize(&self, interactor: &mut spru::interaction::Interactor<BevyLookupMut<'l>, Self::ActionCatalog, item::IdT<player::Root>>, input: Self::In) -> Result<Self::Out, LookupInteractionError<lookup::BevyError, Self::Error>> {
        let deck = interactor.create(pile::Create::new(card::Card::all())).map_err(LookupInteractionError::Lookup)?;
        let discard = interactor.create(pile::Create::new([])).map_err(LookupInteractionError::Lookup)?;
        let round = interactor.create(counter::Create::new(0)).map_err(LookupInteractionError::Lookup)?;

        let root = Root {
            deck,
            discard,
            round,
        };
        interactor.create(verbatim::Create::new(root)).map_err(LookupInteractionError::Lookup)
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub deck: item::IdT<Pile<Card>>,
    pub discard: item::IdT<Pile<Card>>,
    pub round: item::IdT<Counter<u8>>,
}