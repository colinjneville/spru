use std::{marker::PhantomData, sync::Arc};

use crate::{item, Item};

pub type Key = u32;

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct SnapshotType {
    index: Key,
    items: Box<[Item<Box<[u8]>>]>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Snapshot<ItemCatalog, Root> {
    root: item::IdT<Root>,
    items: Arc<[SnapshotType]>,
    #[serde(skip)]
    _p: PhantomData<fn(ItemCatalog) -> ItemCatalog>,
}


impl<ItemCatalog, Root> Snapshot<ItemCatalog, Root> {
    pub(crate) fn new(root: item::IdT<Root>, lookup: &item::lookup::Canonical<ItemCatalog>) -> Result<Self, CreateError> {
        let mut snapshot_items_map = vec![];
        for (&key, item_map) in lookup.items_map().iter() {
            snapshot_items_map.push(SnapshotType {
                index: key,
                items: item_map.as_serialized()?,
            });
        }

        Ok(Self {
            root,
            items: snapshot_items_map.into(),
            _p: Default::default(),
        })
    }

    pub(crate) fn root(&self) -> item::IdT<Root> {
        self.root
    }

    pub(crate) fn apply<Lookup>(&self, lookup: &mut Lookup) -> Result<(), ApplyError<Lookup::Error>> 
    where 
        Lookup: item::Lookup,
        ItemCatalog: item::Catalog<Lookup>,
    {
        for item_type in &*self.items {
            for item in &*item_type.items {
                ItemCatalog::apply_item(item_type.index, item, lookup)?;
            }
        }
        Ok(())
    }
}

// TODO not sure where this should go
#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum CreateError {
    #[error(transparent)]
    Serialization(#[from] rmp_serde::encode::Error),
}

#[doc(hidden)]
#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ApplyError<LookupError> {
    #[error(transparent)]
    Deserialization(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    Lookup(LookupError),
}
