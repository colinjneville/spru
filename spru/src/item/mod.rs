pub mod id;
pub use id::{Id, IdT};
pub mod lookup;
pub use lookup::Lookup;
pub mod version;
pub use version::Version;

use std::ops;

use crate::common;

pub type Index = u32;

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Item<T> {
    id: IdT<T>,
    version: Version,
    state: T,
}

impl<T> Item<T> {
    pub(crate) fn new(id: IdT<T>, version: Version, state: T) -> Self {
        Self {
            id,
            version,
            state,
        }
    }

    // Only to be used by macros for deserialization
    #[doc(hidden)]
    pub fn new_untyped_id(id: Id, version: Version, state: T) -> Self {
        let id = IdT::new(id);
        Self::new(id, version, state)
    }

    pub fn id(self: &Self) -> IdT<T> {
        self.id
    }

    pub fn version(self: &Self) -> Version {
        self.version
    }

    pub fn get(&self) -> &T {
        &self.state
    }

    pub(crate) fn get_mut(&mut self) -> &mut T {
        &mut self.state
    }

    pub(crate) fn into_value(self) -> T {
        self.state
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
        &mut self.state
    }

    pub fn test_version_mut(&mut self) -> &mut Version {
        &mut self.version
    }
}

// TODO deref is not ideal when we have fields, etc.
impl<T> ops::Deref for Item<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[doc(hidden)]
#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Erased {
    id: Id,
    version: Version,
    #[serde(with = "serde_bytes")]
    state: Box<[u8]>,
}

impl Erased {
    pub(crate) fn new<T>(item: &Item<T>) 
        -> Result<Self, common::error::Save> 
    where 
        T: serde::Serialize,
    {
        let Item {
            id,
            version,
            ref state,
        } = *item;
        let id = id.untyped();

        Ok(Self {
            id,
            version,
            state: rmp_serde::to_vec(state)?.into_boxed_slice()
        })
    }

    #[doc(hidden)]
    pub fn cast<Lookup, T>(&self, lookup: &mut Lookup) 
        -> Result<(), common::error::Load> 
    where 
        Lookup: self::Lookup,
        T: lookup::Lookupable<Lookup::State> + serde::de::DeserializeOwned,
    {
        let id = IdT::new(self.id);
        let value = rmp_serde::from_slice::<T>(&*self.state)?;
        let item = Item::new(id, self.version, value);
        lookup.create(item)?;
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct Mut<M>(M);

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
