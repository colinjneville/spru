use std::{any, collections::HashMap};

use spru::{item::{self, IdT}, Item};

#[derive(Debug, Default)]
pub struct Standalone {
    map: HashMap<any::TypeId, Box<dyn any::Any>>,
}

impl Standalone {
    pub fn new() -> Self {
        Self::default()
    }
}

impl spru::item::Lookup for Standalone {
    type Error = Error;
}

impl<T: 'static> spru::item::lookup::OfType<T> for Standalone {
    fn lookup(&self, id: IdT<T>) -> Result<&Item<T>, Self::Error> {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get(&type_id) {
            let inner_map = inner_map.downcast_ref::<HashMap<item::IdT<T>, Item<T>>>()
                .expect("Internal type mismatch");

            inner_map.get(&id)
        } else {
            None
        };
        item.ok_or(Error::IdNotFound(id.untyped().clone()))
    }
}

type InnerMap<T> = HashMap<item::IdT<T>, Item<T>>;

impl<T: 'static> spru::item::lookup::OfTypeMut<T> for Standalone {
    type Mut<'lr> = &'lr mut spru::Item<T>
    where Self: 'lr;

    fn lookup_mut(&mut self, id: IdT<T>) -> Result<Self::Mut<'_>, Self::Error> {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get_mut(&type_id) {
            let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
                .expect("Interal type mismatch");

            inner_map.get_mut(&id)
        } else {
            None
        };
        item.ok_or(Error::IdNotFound(id.untyped().clone()))
    }

    fn create(&mut self, value: spru::Item<T>) -> Result<(), Self::Error> {
        let id = value.id();
        let type_id = any::TypeId::of::<T>();
        let inner_map = self.map.entry(type_id)
            .or_insert(Box::new(InnerMap::<T>::new()));

        let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
            .expect("Interal type mismatch");
        if inner_map.contains_key(&id) {
            Err(Error::IdAlreadyExists(id.untyped().clone()))
        } else {
            inner_map.insert(id.clone(), value);
            Ok(())
        }
    }

    fn destroy(&mut self, id: IdT<T>) -> Result<spru::Item<T>, Self::Error> {
        let type_id = any::TypeId::of::<T>();
        let item = if let Some(inner_map) = self.map.get_mut(&type_id) {
            let inner_map = inner_map.downcast_mut::<InnerMap<T>>()
                .expect("Interal type mismatch");

            inner_map.remove(&id)
        } else {
            None
        };

        item.ok_or(Error::IdNotFound(id.untyped().clone()))
    }
}

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
    use item::lookup::OfType;

    use super::*;

    #[test]
    fn standalone() {
        use spru::item::lookup::OfTypeMut as _;
        let mut lookup = Standalone::new();

        let id0 = spru::item::IdT::test_new(0);
        let id1 = spru::item::IdT::test_new(1);
        let id2 = spru::item::IdT::test_new(2);

        lookup.create(spru::Item::test_new(id0.clone(), "zero"))
            .unwrap();

        lookup.create(spru::Item::test_new(id1.clone(), "one"))
            .unwrap();

        lookup.create(spru::Item::test_new(id2.clone(), "two"))
            .unwrap();

        let s0 = lookup.lookup_mut(id0)
            .unwrap()
            .test_get_mut();

        *s0 = "ZERO";

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