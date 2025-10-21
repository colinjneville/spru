use std::{marker::PhantomData, sync::Arc};

use crate::{common, item};


#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct SnapshotType {
    index: u32,
    items: Box<[item::Erased]>,
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
    pub(crate) fn new(root: Root, lookup: &item::lookup::Canonical<State>) -> Result<Self, common::error::Save> {
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

    pub(crate) fn apply<Lookup>(&self, lookup: &mut Lookup) -> Result<(), common::error::Load> 
    where 
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
