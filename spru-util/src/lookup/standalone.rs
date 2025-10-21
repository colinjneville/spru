use std::{any, collections::HashMap, marker::PhantomData};

use derive_where::derive_where;
use spru::{item::{self, IdT}, Item};

#[derive_where(Debug, Default; )]
pub struct Standalone<State> {
    map: HashMap<any::TypeId, Box<dyn any::Any>>,
    _state: PhantomData<fn() -> State>,
}

impl<State> Standalone<State> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<State: spru::State> spru::item::Lookup for Standalone<State> {
    type State = State;

    fn lookup<T>(&self, id: IdT<T>) 
        -> spru::item::lookup::Result<&Item<T>> 
    where
        T: item::lookup::Lookupable<Self::State>,
    {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get(&type_id) {
            let inner_map = inner_map.downcast_ref::<HashMap<item::IdT<T>, Item<T>>>()
                .expect("Internal type mismatch");

            inner_map.get(&id)
        } else {
            None
        };
        item.ok_or(Error::IdNotFound(id.untyped().clone()))
            .map_err(Into::into)
    }

    #[allow(refining_impl_trait)]
    fn lookup_mut<T>(&mut self, id: IdT<T>) 
        -> spru::item::lookup::Result<&mut spru::Item<T>> 
    where
        T: item::lookup::Lookupable<Self::State>,    
    {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get_mut(&type_id) {
            let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
                .expect("Interal type mismatch");

            inner_map.get_mut(&id)
        } else {
            None
        };
        item.ok_or(Error::IdNotFound(id.untyped().clone()))
            .map_err(Into::into)
    }

    fn create<T>(&mut self, value: spru::Item<T>) 
        -> spru::item::lookup::Result<()> 
    where
        T: item::lookup::Lookupable<Self::State>,       
    {
        let id = value.id();
        let type_id = any::TypeId::of::<T>();
        let inner_map = self.map.entry(type_id)
            .or_insert(Box::new(InnerMap::<T>::new()));

        let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
            .expect("Interal type mismatch");
        if inner_map.contains_key(&id) {
            Err(Error::IdAlreadyExists(id.untyped().clone()).into())
        } else {
            inner_map.insert(id.clone(), value);
            Ok(())
        }
    }

    fn destroy<T>(&mut self, id: IdT<T>) 
        -> spru::item::lookup::Result<spru::Item<T>> 
    where
        T: item::lookup::Lookupable<Self::State>,       
    {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get_mut(&type_id) {
            let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
                .expect("Interal type mismatch");

            inner_map.remove(&id)
        } else {
            None
        };

        item.ok_or(Error::IdNotFound(id.untyped().clone()).into())
    }
}

type InnerMap<T> = HashMap<item::IdT<T>, Item<T>>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Id '{0}' was not found")]
    IdNotFound(item::Id),
    #[error("Id '{0}' already exists")]
    IdAlreadyExists(item::Id),
}

#[cfg(test)]
mod test {
    use super::*;

    #[tagset::tagset(impl spru::State)]
    #[tagset(String)]
    struct State;

    #[test]
    fn standalone() {
        use spru::item::lookup::Lookup as _;
        let mut lookup = Standalone::<State>::new();

        let id0 = spru::item::IdT::test_new(0);
        let id1 = spru::item::IdT::test_new(1);
        let id2 = spru::item::IdT::test_new(2);

        lookup.create(spru::Item::test_new(id0.clone(), "zero".to_string()))
            .unwrap();

        lookup.create(spru::Item::test_new(id1.clone(), "one".to_string()))
            .unwrap();

        lookup.create(spru::Item::test_new(id2.clone(), "two".to_string()))
            .unwrap();

        let s0 = lookup.lookup_mut(id0)
            .unwrap()
            .test_get_mut();

        *s0 = "ZERO".to_string();

        let s0 = lookup.lookup(id0)
            .unwrap()
            .get();
        assert_eq!(s0, &"ZERO");

        let s1 = lookup.lookup(id1)
            .unwrap()
            .get();
        assert_eq!(s1, &"one");

        lookup.destroy(id2)
            .unwrap();
    }
}