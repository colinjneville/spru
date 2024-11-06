pub mod catalog;
pub use catalog::Catalog;
pub mod id;
pub use id::{Id, IdT};
pub mod lookup;
pub use lookup::Lookup;
pub mod version;
pub use version::Version;

use std::ops;

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Item<T> {
    id: IdT<T>,
    version: Version,
    data: T,
}

impl<T> Item<T> {
    pub(crate) fn new(id: IdT<T>, version: Version, data: T) -> Self {
        Self {
            id,
            version,
            data,
        }
    }

    // Only to be used by macros for deserialization
    #[doc(hidden)]
    pub fn new_untyped_id(id: Id, version: Version, data: T) -> Self {
        let id = IdT::new(id);
        Self::new(id, version, data)
    }

    pub fn id(self: &Self) -> &IdT<T> {
        &self.id
    }

    pub fn version(self: &Self) -> Version {
        self.version
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub(crate) fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub(crate) fn into_value(self) -> T {
        self.data
    }

    pub(crate) fn set_version(&mut self, version: Version) {
        self.version = version;
    }
}

#[cfg(feature = "test-util")]
#[doc(hidden)]
impl<T> Item<T> {
    pub fn test_new(id: IdT<T>, value: T) -> Self {
        Self::new(id, Version::ZERO, value)
    }

    pub fn test_zero(value: T) -> Self {
        Self::new(IdT::test_zero(), Version::ZERO, value)
    }

    pub fn test_get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn test_version_mut(&mut self) -> &mut Version {
        &mut self.version
    }
}

// TODO deref is not ideal when we have fields, etc.
impl<T> ops::Deref for Item<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub struct Mut<M>(M);

impl<M> Mut<M> {
    pub(crate) fn new(m: M) -> Self {
        Self(m)
    }
}

#[cfg(feature = "test-util")]
#[doc(hidden)]
impl<M> Mut<M> {
    pub fn test_new(m: M) -> Self {
        Self(m)
    }
}

impl<T, M: ops::DerefMut<Target=Item<T>>> ops::Deref for Mut<M> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl<T, M: ops::DerefMut<Target=Item<T>>> ops::DerefMut for Mut<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Item::get_mut(&mut *self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn catalog() {
        extern crate self as spru;

        #[repr(u32)]
        #[derive(crate::item::Catalog)]
        enum MyCatalog {
            A(bool),
            B(u8),
            C(u16) = 7,
            D(u32),
            E(u64) = 1 + 1,
        }

        // assert!(registry.0.contains_key(&0)); // A
        // assert!(registry.0.contains_key(&1)); // B
        // assert!(registry.0.contains_key(&7)); // C
        // assert!(registry.0.contains_key(&8)); // D
        // assert!(registry.0.contains_key(&2)); // E
    }
}