use std::collections::VecDeque;

use crate::{action, item::{self}, log};


#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, ActionCatalogError> {
    #[error(transparent)]
    Lookup(LookupError),
    #[error(transparent)]
    Item(item::id::Error),
    #[error(transparent)]
    Version(item::version::MismatchError),
    #[error(transparent)]
    ActionCatalog(ActionCatalogError),
}

impl<LookupError, ActionCatalogError> From<action::catalog::Error<LookupError, ActionCatalogError>> for Error<LookupError, ActionCatalogError> {
    fn from(value: action::catalog::Error<LookupError, ActionCatalogError>) -> Self {
        match value {
            action::catalog::Error::Lookup(e) => Self::Lookup(e),
            action::catalog::Error::Item(e) => Self::Item(e),
            action::catalog::Error::Version(e) => Self::Version(e),
            action::catalog::Error::Action(e) => Self::ActionCatalog(e),
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Packed<ActionCatalog> {
    item_id: item::Id,
    version_change: item::version::Change,
    action: ActionCatalog,
    appended_actions: Vec<ActionCatalog>,
}

impl<ActionCatalog> Packed<ActionCatalog> {
    pub(crate) fn new(item_id: item::Id, version_change: item::version::Change, action: ActionCatalog) -> Self {
        Self {
            item_id,
            version_change,
            action,
            appended_actions: vec![],
        }
    }

    pub fn append(&mut self, action: ActionCatalog) {
        self.appended_actions.push(action);
    }

    pub(crate) fn item_id(&self) -> item::Id {
        self.item_id
    }

    pub(crate) fn version_change(&self) -> item::version::Change {
        self.version_change
    }

    pub(crate) fn expand(&self) -> impl Iterator<Item = Record<ActionCatalog>> {
        (0..self.appended_actions.len() + 1).into_iter()
            .map(|i| Record::new(self, i))
    }

    pub(crate) fn into_action(self) -> ActionCatalog {
        self.action
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Record<'r, ActionCatalog> {
    packed: &'r Packed<ActionCatalog>,
    index: usize,
}

impl<'r, ActionCatalog> Record<'r, ActionCatalog> {
    pub(crate) fn new(packed: &'r Packed<ActionCatalog>, index: usize) -> Self {
        Self {
            packed,
            index,
        }
    }

    fn apply_internal<Lookup: item::Lookup>(&self, lookup: &mut Lookup) -> Result<Option<ActionCatalog>, Error<Lookup::Error, ActionCatalog::Error>> 
    where 
        ActionCatalog: action::Catalog<Lookup> 
    {
        let undo = self.action().apply(action::adapter::Data::new(lookup, self.item_id().clone(), self.version_change()))?;
        Ok(undo)
    }

    pub(crate) fn item_id(&self) -> item::Id {
        self.packed.item_id
    }

    pub(crate) fn version_change(&self) -> item::version::Change {
        self.packed.version_change
    }

    pub(crate) fn action(&self) -> &ActionCatalog {
        match self.index.checked_sub(1) {
            Some(index) => &self.packed.appended_actions[index],
            None => &self.packed.action,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Records<ActionCatalog> {
    records: VecDeque<Packed<ActionCatalog>>,
}

impl<ActionCatalog> Records<ActionCatalog> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, record: Packed<ActionCatalog>) {
        self.records.push_back(record);
    }

    pub fn iter(&self) -> impl Iterator<Item = Record<ActionCatalog>> {
        self.records.iter()
            .flat_map(Packed::expand)
    }
}

impl<ActionCatalog> Default for Records<ActionCatalog> {
    fn default() -> Self {
        Self { records: Default::default() }
    }
}

impl<ActionCatalog> Records<ActionCatalog> {
    pub fn apply<'l, Lookup: item::Lookup>(&self, lookup: &'l mut Lookup) 
        -> Result<Self, Error<Lookup::Error, ActionCatalog::Error>> 
    where 
        ActionCatalog: action::Catalog<Lookup> 
    {
        self.apply_internal(lookup)
            .map_err(|(_, e)| e)
    }

    pub fn apply_or_revert<'l, Lookup: item::Lookup>(&self, lookup: &'l mut Lookup) 
        -> Result<Self, log::Error<Lookup::Error, ActionCatalog::Error>> 
    where 
        ActionCatalog: action::Catalog<Lookup> 
    {
        match self.apply_internal(lookup) {
            Ok(undo) => Ok(undo),
            Err((undo, e)) => {
                if let Err(e2) = undo.apply(lookup) {
                    Err(log::Error::Revert(log::RevertError { initial: Some(e), fatal: e2 }))
                } else {
                    Err(log::Error::Record(e))
                }
            }
        }
    }

    fn apply_internal<'l, Lookup: item::Lookup>(&self, lookup: &'l mut Lookup) 
        -> Result<Self, (Self, Error<Lookup::Error, ActionCatalog::Error>)> 
    where 
        ActionCatalog: action::Catalog<Lookup> 
    {
        let mut undo = Self::default();

        for r in self.iter() {
            match r.apply_internal(lookup) {
                Ok(Some(undo_action)) => {
                    if let Some(back) = undo.records.back_mut() {
                        // Pack the undo records if the original record was packed
                        if back.item_id == r.item_id() {
                            back.append(undo_action);
                            continue;
                        }

                        // Otherwise start a new undo record
                        let packed = Packed::new(r.item_id(), r.version_change().undo(), undo_action);
                        undo.records.push_back(packed);
                    }
                }
                Ok(None) => { }
                Err(e) => return Err((undo, e)),
            }
        }

        Ok(undo)
    }
}
