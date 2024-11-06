use std::{any, fmt, marker::PhantomData};

use crate::{snapshot, item, type_index, Item};

#[derive(Debug)]
pub struct Canonical<Catalog> {
    items_map: ItemsMap<Catalog>,
}

impl<Catalog> Canonical<Catalog> {
    pub(crate) fn new() -> Self {
        Self {
            items_map: ItemsMap::new(),
        }
    }

    pub(crate) fn items_map(&self) -> &ItemsMap<Catalog> {
        &self.items_map
    }
}

impl<Catalog> item::Lookup for Canonical<Catalog> {
    type Error = Error;
}

impl<Catalog, T> item::lookup::OfType<T> for Canonical<Catalog> 
where 
    Catalog: type_index::TypeToU32<T>,
    T: any::Any,
{
    fn lookup(&self, id: &item::IdT<T>) -> Result<&Item<T>, Self::Error> {
        self.items_map.get(id)
            .ok_or(Error::Temp)
    }
}

impl<Catalog, T> item::lookup::OfTypeMut<T> for Canonical<Catalog> 
where 
    Catalog: type_index::TypeToU32<T>,
    T: any::Any + serde::Serialize,
{
    type Mut<'lr> = &'lr mut Item<T>
    where Self: 'lr;

    fn lookup_mut(&mut self, id: &item::IdT<T>) -> Result<Self::Mut<'_>, Self::Error> {
        self.items_map.get_mut(id)
            .ok_or(Error::Temp)
    }

    fn create(&mut self, value: Item<T>) -> Result<(), Self::Error> {
        self.items_map.insert(value);
        Ok(())
    }

    fn destroy(&mut self, id: &item::IdT<T>) -> Result<Item<T>, Self::Error> {
        self.items_map.remove(id)
            .ok_or(Error::Temp)
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
        let Self {
            map,
        } = self;
        
        f.debug_list()
            .entries(map.keys())
            .finish()
    }
}

impl<T> serde::Serialize for ItemMap<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
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
        D: serde::Deserializer<'de>
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
                    map.insert(*item.id().untyped(), item);
                }

                Ok(ItemMap {
                    map,
                })
            }
        }

        deserializer.deserialize_seq(Visitor(Default::default()))
    }
}

impl<T> ItemMap<T> {
    pub fn insert(&mut self, item: Item<T>) -> Option<Item<T>> {
        self.map.insert(*item.id().untyped(), item)
    }

    pub fn remove(&mut self, id: &item::Id) -> Option<Item<T>> {
        self.map.remove(&id)
    }

    pub fn get(&self, id: &item::Id) -> Option<&Item<T>> {
        self.map.get(id)
    }

    pub fn get_mut(&mut self, id: &item::Id) -> Option<&mut Item<T>> {
        self.map.get_mut(id)
    }
}

pub(crate) trait ErasedItemMap: any::Any + fmt::Debug {
    // TODO: upcasting stabilzing soon?
    // https://github.com/rust-lang/rust/issues/65991
    fn as_any(&self) -> &dyn any::Any;

    fn as_any_mut(&mut self) -> &mut dyn any::Any;

    fn into_any(self: Box<Self>) -> Box<dyn any::Any>;

    fn as_serialized(&self) -> Result<Box<[Item<Box<[u8]>>]>, snapshot::CreateError>;
}

impl<T> ErasedItemMap for ItemMap<T> 
where T: any::Any + serde::Serialize {
    fn as_any(&self) -> &dyn any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn any::Any> {
        self
    }
    
    fn as_serialized(&self) -> Result<Box<[Item<Box<[u8]>>]>, snapshot::CreateError> {
        let items = self.map.values()
            .map(map_item)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items.into_boxed_slice())
    }
}

fn map_item<T>(item: &Item<T>) -> Result<Item<Box<[u8]>>, snapshot::CreateError> 
where T: serde::Serialize {
    let id = item::IdT::new(*item.id().untyped());
    let version = item.version();
    let item = Item::new(id, version, rmp_serde::to_vec(&item.get())?.into_boxed_slice());
    Ok(item)
}

#[derive(Debug)]
pub(crate) struct ItemsMap<Catalog> {
    raw: std::collections::HashMap<u32, Box<dyn ErasedItemMap>>,
    _p: PhantomData<fn(Catalog) -> Catalog>,
}

impl<Catalog> ItemsMap<Catalog> {
    pub fn new() -> Self {
        Self {
            raw: Default::default(),
            _p: PhantomData,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &Box<dyn ErasedItemMap>)> {
        self.raw.iter()
    }
    
    pub fn insert<T>(&mut self, item: Item<T>) 
    where 
        T: any::Any + serde::Serialize,
        Catalog: type_index::TypeToU32<T>,
    {
        let index = Catalog::N;
        let item_map = self.raw.entry(index)
            .or_insert_with(|| Box::new(ItemMap::<T>::new()));
        let item_map = item_map.as_any_mut().downcast_mut::<ItemMap<T>>()
            .expect("Type map is invalid");
        let prev = item_map.insert(item);
        assert!(prev.is_none(), "Item id was already in type map");
    }

    pub fn remove<T>(&mut self, item_id: &item::IdT<T>) -> Option<Item<T>> 
    where 
        T: any::Any,
        Catalog: type_index::TypeToU32<T>,
    {
        let index = Catalog::N;
        if let Some(item_map) = self.raw.get_mut(&index) {
            let item_map = item_map.as_any_mut().downcast_mut::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.remove(item_id)
        } else {
            None
        }
    }

    pub fn get<T>(&self, item_id: &item::IdT<T>) -> Option<&Item<T>> 
    where 
        T: any::Any,
        Catalog: type_index::TypeToU32<T>,
    {
        let index = Catalog::N;
        if let Some(item_map) = self.raw.get(&index) {
            let item_map = item_map.as_any().downcast_ref::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.get(item_id)
        } else {
            None
        }
    }

    pub fn get_mut<T>(&mut self, item_id: &item::IdT<T>) -> Option<&mut Item<T>> 
    where 
        T: any::Any,
        Catalog: type_index::TypeToU32<T>,
    {
        let index = Catalog::N;
        if let Some(item_map) = self.raw.get_mut(&index) {
            let item_map = item_map.as_any_mut().downcast_mut::<ItemMap<T>>()
                .expect("Type map is invalid");
            item_map.get_mut(item_id)
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    // TODO
    #[error("TODO")]
    Temp
}

#[cfg(test)]
mod test {
    use crate::transaction;

    use super::*;

    #[test]
    fn round_trip() {
        use item::lookup::OfTypeMut as _;

        extern crate self as spru;

        #[derive(Debug)]
        #[derive(item::Catalog)]
        #[repr(u32)]
        enum MyCatalog {
            I32(i32) = 5,
            I64(i64) = 7,
        }

        let mut id = item::Id::new();
        let mut canonical = Canonical::<MyCatalog>::new();

        canonical.create(Item::new_untyped_id(id, item::Version::ZERO, 1i32)).expect("create failed");
        id = id.next();
        canonical.create(Item::new_untyped_id(id, item::Version::ZERO, 2i32)).expect("create failed");
        id = id.next();
        canonical.create(Item::new_untyped_id(id, item::Version::ZERO, 3i64)).expect("create failed");
        id = id.next();
        canonical.create(Item::new_untyped_id(id, item::Version::ZERO, 4i64)).expect("create failed");
        
        
        let checkpoint = Snapshot::new(item::Id::new().force_type::<()>(), &canonical).expect("checkpoint failed");

        let mut canonical2 = Canonical::<MyCatalog>::new();
        checkpoint.apply(&mut canonical2).expect("checkpoint apply failed");

        let mut id = item::Id::new();
        assert_eq!(canonical2.items_map.get::<i32>(&id.force_type()).expect("lookup failed").get(), &1i32);
        id = id.next();
        assert_eq!(canonical2.items_map.get::<i32>(&id.force_type()).expect("lookup failed").get(), &2i32);
        id = id.next();
        assert_eq!(canonical2.items_map.get::<i64>(&id.force_type()).expect("lookup failed").get(), &3i64);
        id = id.next();
        assert_eq!(canonical2.items_map.get::<i64>(&id.force_type()).expect("lookup failed").get(), &4i64);
    }
}
