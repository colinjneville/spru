use std::{marker::PhantomData, sync::Arc};

use derive_where::derive_where;

use crate::{common, item};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SnapshotType<Index> {
    index: Index,
    items: Box<[item::Erased]>,
}

#[derive_where(Debug, Serialize, Deserialize; Repr, Root)]
pub(crate) struct Snapshot<Repr, State, Root> {
    root: Root,
    items: Arc<[SnapshotType<Repr>]>,
    #[serde(skip)]
    _p: PhantomData<State>,
}

impl<State: tagset::TagSet<Repr: Clone>, Root> Snapshot<State::Repr, State, Root> {
    pub(crate) fn new(
        root: Root,
        storage: &item::storage::Canonical<State::Repr, State>,
    ) -> Result<Self, common::error::Save> {
        let mut snapshot_items_map = vec![];
        for (key, item_map) in storage.items_map().iter() {
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

    pub(crate) fn apply<Storage>(&self, storage: &mut Storage) -> Result<(), common::error::Load>
    where
        State: crate::State,
        Storage: item::Storage<State = State>,
    {
        for item_type in &*self.items {
            for item in &*item_type.items {
                State::apply_state(item_type.index.clone(), item, storage)?;
            }
        }
        Ok(())
    }
}
