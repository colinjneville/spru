use std::{marker::PhantomData, sync::Arc};

use crate::{item::{self, lookup}, Item};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct SnapshotType {
    index: u32,
    items: Box<[Item<Box<[u8]>>]>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Snapshot<State, Root> {
    root: Root,
    items: Arc<[SnapshotType]>,
    #[serde(skip)]
    _p: PhantomData<fn(State) -> State>,
}


impl<State, Root> Snapshot<State, Root> {
    pub(crate) fn new(root: Root, lookup: &item::lookup::Canonical<State>) -> Result<Self, CreateError> {
        let mut snapshot_items_map = vec![];
        for (key, item_map) in lookup.items_map().iter() {
            snapshot_items_map.push(SnapshotType {
                index: key.clone(),
                items: item_map.as_serialized()?,
            });
        }

        Ok(Self {
            root,
            items: snapshot_items_map.into(),
            _p: Default::default(),
        })
    }

    pub(crate) fn root(&self) -> &Root {
        &self.root
    }

    pub(crate) fn apply<Lookup>(&self, lookup: &mut Lookup) -> Result<(), ApplyError> 
    where 
        // State: crate::State<Lookup, Repr: TryFrom<state::Index>>,
        State: crate::State,
        Lookup: item::Lookup<State = State>,
    {
        for item_type in &*self.items {
            for item in &*item_type.items {
                let Ok(index) = item_type.index.try_into() else {
                    // TODO This should be a proper error
                    unimplemented!("Index could not be converted back to repr type");
                };
                
                State::apply_state(index, item, lookup)?;
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
pub enum ApplyError {
    #[error(transparent)]
    Deserialization(#[from] rmp_serde::decode::Error),
    #[error("{0}")]
    Lookup(lookup::Error),
}

impl From<lookup::Error> for ApplyError {
    fn from(value: lookup::Error) -> Self {
        Self::Lookup(value)
    }
}
