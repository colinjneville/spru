
use crate::{action, init, interaction::Interactor, item::{self}, player, Init};


#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InitializeError<PlayerInitError> {
    Lookup(item::lookup::canonical::Error),
    Init(PlayerInitError),
}

impl<PlayerInitError> From<init::Error<PlayerInitError>> for InitializeError<PlayerInitError> {
    fn from(value: init::Error<PlayerInitError>) -> Self {
        match value {
            init::Error::Lookup(e) => Self::Lookup(e),
            init::Error::Init(e) => Self::Init(e),
        }
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Manager<PlayerInit> {
    init: PlayerInit,
    reservation_range: Vec<item::id::Range>,
}

impl<PlayerInit> Manager<PlayerInit> {
    pub(crate) fn new(init: PlayerInit) -> Self {
        Self {
            init,
            reservation_range: vec![],
        }
    }

    pub(crate) fn initialize<ItemCatalog, ActionCatalog, Root>(
        &self, 
        interactor: &mut Interactor<item::lookup::Canonical<ItemCatalog>, ActionCatalog, Root>, 
        input: PlayerInit::In
    ) -> Result<(), InitializeError<PlayerInit::Error>>
    where 
        ActionCatalog: action::Catalog<item::lookup::Canonical<ItemCatalog>>,
        PlayerInit: crate::Init<ItemCatalog, ActionCatalog, Root, Out = ()>, 
    {
        self.init.initialize(interactor, input)?;
        Ok(())
    }

    pub(crate) fn add(&mut self, reservation_range: item::id::Range) -> player::Id {
        let id = player::Id(self.reservation_range.len());
        self.reservation_range.push(reservation_range);
        
        id
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = player::Id> {
        (0..self.reservation_range.len())
            .into_iter()
            .map(|i| player::Id(i))
    }
}
