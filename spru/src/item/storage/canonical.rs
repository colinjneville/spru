use std::{any, fmt, marker::PhantomData};

use derive_where::derive_where;

use crate::{
    Item, common,
    item::{self, storage},
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Item {0} does not exist")]
    ItemDoesNotExist(item::Id),
}

/// The implementation of [item::Storage] which runs on [Server](crate::Server)s.
/// Users do not need to interact directly with this, but it shows up in
/// trait signatures.
#[derive_where(Debug; Repr)]
pub struct Canonical<Repr, State> {
    items_map: ItemsMap<Repr, State>,
}

impl<Repr, State> Canonical<Repr, State> {
    pub(crate) fn new() -> Self {
        Self {
            items_map: ItemsMap::new(),
        }
    }

    pub(crate) fn items_map(&self) -> &ItemsMap<Repr, State> {
        &self.items_map
    }
}

impl<State: crate::State> item::ReadOnlyStorage for Canonical<State::Repr, State> {
    type State = State;

    fn get<T>(&self, id: item::IdT<T>) -> Result<&Item<T>, storage::Error>
    where
        T: super::Storable<Self::State>,
    {
        self.items_map
            .get(id)
            .ok_or(Error::ItemDoesNotExist(id.untyped()))
            .map_err(Into::into)
    }
}

impl<State: crate::State> item::Storage for Canonical<State::Repr, State> {
    #[allow(refining_impl_trait)]
    fn get_mut<T>(&mut self, id: item::IdT<T>) -> Result<&mut Item<T>, storage::Error>
    where
        T: super::Storable<Self::State>,
    {
        self.items_map
            .get_mut(id)
            .ok_or(Error::ItemDoesNotExist(id.untyped()))
            .map_err(Into::into)
    }

    fn create<T>(&mut self, value: Item<T>) -> Result<(), storage::Error>
    where
        T: super::Storable<Self::State>,
    {
        self.items_map.insert(value);
        Ok(())
    }

    fn destroy<T>(&mut self, id: item::IdT<T>) -> Result<Item<T>, storage::Error>
    where
        T: super::Storable<Self::State>,
    {
        self.items_map
            .remove(id)
            .ok_or(Error::ItemDoesNotExist(id.untyped()))
            .map_err(Into::into)
    }
}

struct ItemMap<T> {
    map: halfbrown::SizedHashMap<item::Id, Item<T>, halfbrown::DefaultHashBuilder, 16>,
}

impl<T> ItemMap<T> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<T> fmt::Debug for ItemMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { map } = self;

        f.debug_list().entries(map.keys()).finish()
    }
}

impl<T> serde::Serialize for ItemMap<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // The id key is already stored in the Item value
        serializer.collect_seq(self.map.values())
    }
}

impl<'de, T> serde::Deserialize<'de> for ItemMap<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<T>);
        impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
        where
            T: serde::Deserialize<'de>,
        {
            type Value = ItemMap<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "an Item")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut map = halfbrown::SizedHashMap::new();
                while let Some(item) = seq.next_element::<Item<T>>()? {
                    map.insert(item.id().untyped(), item);
                }

                Ok(ItemMap { map })
            }
        }

        deserializer.deserialize_seq(Visitor(Default::default()))
    }
}

impl<T> ItemMap<T> {
    pub fn insert(&mut self, item: Item<T>) -> Option<Item<T>> {
        self.map.insert(item.id().untyped(), item)
    }

    pub fn remove(&mut self, id: item::Id) -> Option<Item<T>> {
        self.map.remove(&id)
    }

    pub fn get(&self, id: item::Id) -> Option<&Item<T>> {
        self.map.get(&id)
    }

    pub fn get_mut(&mut self, id: item::Id) -> Option<&mut Item<T>> {
        self.map.get_mut(&id)
    }
}

pub(crate) trait ErasedItemMap: any::Any + fmt::Debug + Send + Sync {
    fn as_serialized(&self) -> Result<Box<[item::Erased]>, common::error::Save>;
}

impl<T> ErasedItemMap for ItemMap<T>
where
    T: any::Any + serde::Serialize + Send + Sync,
{
    fn as_serialized(&self) -> Result<Box<[item::Erased]>, common::error::Save> {
        let items = self
            .map
            .values()
            .map(item::Erased::new)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items.into_boxed_slice())
    }
}

#[derive_where(Debug; Repr)]
pub(crate) struct ItemsMap<Repr, State> {
    raw: std::collections::HashMap<Repr, Box<dyn ErasedItemMap>>,
    _p: PhantomData<fn(State) -> State>,
}

impl<Repr, State> ItemsMap<Repr, State> {
    pub fn new() -> Self {
        Self {
            raw: Default::default(),
            _p: PhantomData,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Repr, &Box<dyn ErasedItemMap>)> {
        self.raw.iter()
    }
}

impl<State: crate::State> ItemsMap<State::Repr, State> {
    pub fn insert<T>(&mut self, item: Item<T>)
    where
        T: any::Any + serde::Serialize + Send + Sync,
        State: tagset::TagSetDiscriminant<T, Repr: Eq + std::hash::Hash>,
    {
        let index = State::DISCRIMINANT;
        let item_map = self
            .raw
            .entry(index)
            .or_insert_with(|| Box::new(ItemMap::<T>::new()));
        let item_map = (&mut **item_map as &mut dyn any::Any)
            .downcast_mut::<ItemMap<T>>()
            .expect("Type map is invalid");
        let prev = item_map.insert(item);
        assert!(prev.is_none(), "Item id was already in type map");
    }

    pub fn remove<T>(&mut self, item_id: item::IdT<T>) -> Option<Item<T>>
    where
        T: any::Any,
        State: tagset::TagSetDiscriminant<T, Repr: Eq + std::hash::Hash>,
    {
        let index = State::DISCRIMINANT;
        if let Some(item_map) = self.raw.get_mut(&index) {
            let item_map = (&mut **item_map as &mut dyn any::Any)
                .downcast_mut::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.remove(item_id.untyped())
        } else {
            None
        }
    }

    pub fn get<T>(&self, item_id: item::IdT<T>) -> Option<&Item<T>>
    where
        T: any::Any,
        State: tagset::TagSetDiscriminant<T, Repr: Eq + std::hash::Hash>,
    {
        let index = State::DISCRIMINANT;
        if let Some(item_map) = self.raw.get(&index) {
            let item_map = (&**item_map as &dyn any::Any)
                .downcast_ref::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.get(item_id.untyped())
        } else {
            None
        }
    }

    pub fn get_mut<T>(&mut self, item_id: item::IdT<T>) -> Option<&mut Item<T>>
    where
        T: any::Any,
        State: tagset::TagSetDiscriminant<T, Repr: Eq + std::hash::Hash>,
    {
        let index = State::DISCRIMINANT;
        if let Some(item_map) = self.raw.get_mut(&index) {
            let item_map = (&mut **item_map as &mut dyn any::Any)
                .downcast_mut::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.get_mut(item_id.untyped())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::common;
    use tagset::tagset;

    use super::*;

    extern crate self as spru;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct S0(i32);

    #[derive(serde::Serialize, serde::Deserialize)]
    struct S1(i64);

    #[tagset(impl crate::State)]
    #[tagset(index(5u32))]
    #[tagset(S0)]
    #[tagset(reserved(..7))]
    #[tagset(S1)]
    struct MyCatalog;

    #[test]
    fn round_trip() {
        use crate::item::IdT;
        use item::storage::Storage as _;

        extern crate self as spru;

        let mut id = item::Id::new();
        let mut canonical = Canonical::<u32, MyCatalog>::new();

        canonical
            .create(Item::new(IdT::new(id), item::Version::ZERO, S0(1i32)))
            .expect("create failed");
        id = id.next();
        canonical
            .create(Item::new(IdT::new(id), item::Version::ZERO, S0(2i32)))
            .expect("create failed");
        id = id.next();
        canonical
            .create(Item::new(IdT::new(id), item::Version::ZERO, S1(3i64)))
            .expect("create failed");
        id = id.next();
        canonical
            .create(Item::new(IdT::new(id), item::Version::ZERO, S1(4i64)))
            .expect("create failed");

        let checkpoint = common::Snapshot::new(item::Id::new().force_type::<()>(), &canonical)
            .expect("checkpoint failed");

        let mut canonical2 = Canonical::<u32, MyCatalog>::new();
        checkpoint
            .apply(&mut canonical2)
            .expect("checkpoint apply failed");

        let mut id = item::Id::new();
        assert_eq!(
            canonical2
                .items_map
                .get::<S0>(id.force_type())
                .expect("storage failed")
                .get()
                .0,
            1i32
        );
        id = id.next();
        assert_eq!(
            canonical2
                .items_map
                .get::<S0>(id.force_type())
                .expect("storage failed")
                .get()
                .0,
            2i32
        );
        id = id.next();
        assert_eq!(
            canonical2
                .items_map
                .get::<S1>(id.force_type())
                .expect("storage failed")
                .get()
                .0,
            3i64
        );
        id = id.next();
        assert_eq!(
            canonical2
                .items_map
                .get::<S1>(id.force_type())
                .expect("storage failed")
                .get()
                .0,
            4i64
        );
    }
}
