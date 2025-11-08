pub mod id;
pub use id::{Id, IdT};
pub mod storage;
pub use storage::Storage;
pub mod version;
pub use version::Version;

use std::ops;

use crate::common;

/// Holds one of the types from [trait@crate::State] along with metadata.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Item<T> {
    id: IdT<T>,
    version: Version,
    state: T,
}

impl<T> Item<T> {
    pub(crate) fn new(id: IdT<T>, version: Version, state: T) -> Self {
        Self { id, version, state }
    }

    /// The [IdT] that uniquely identifies this Item
    pub fn id(&self) -> IdT<T> {
        self.id
    }

    pub(crate) fn version(&self) -> Version {
        self.version
    }

    /// The `T` value contained within this Item
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
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Erased {
    id: Id,
    version: Version,
    #[serde(with = "serde_bytes")]
    state: Box<[u8]>,
}

impl Erased {
    pub(crate) fn new<T>(item: &Item<T>) -> Result<Self, common::error::Save>
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
            state: rmp_serde::to_vec(state)?.into_boxed_slice(),
        })
    }

    #[doc(hidden)]
    pub fn cast<Storage, T>(&self, storage: &mut Storage) -> Result<(), common::error::Load>
    where
        Storage: self::Storage,
        T: storage::Storable<Storage::State> + serde::de::DeserializeOwned,
    {
        let id = IdT::new(self.id);
        let value = rmp_serde::from_slice::<T>(&self.state)?;
        let item = Item::new(id, self.version, value);
        storage.create(item)?;

        Ok(())
    }
}

/// The [Item] does not have the expected status
#[derive(Debug, thiserror::Error)]
#[error("{item} {kind}")]
pub struct Error {
    /// The [Item]'s [Id]
    pub item: Id,
    /// The [error::Kind] of [Item] error
    #[source]
    pub kind: error::Kind,
}

impl Error {
    // This is currently covered by Storage errors, which is not ideal
    #[allow(dead_code)]
    pub(crate) fn does_not_exist(item: Id, expected: Version) -> Self {
        Self {
            item,
            kind: error::Kind::DoesNotExist { expected },
        }
    }

    pub(crate) fn already_exists(item: Id, actual: Version) -> Self {
        Self {
            item,
            kind: error::Kind::AlreadyExists { actual },
        }
    }

    pub(crate) fn wrong_version(item: Id, expected: Version, actual: Version) -> Self {
        Self {
            item,
            kind: error::Kind::WrongVersion { expected, actual },
        }
    }

    pub(crate) fn expected_change(item: Id) -> Self {
        Self {
            item,
            kind: error::Kind::ExpectedChange,
        }
    }

    pub(crate) fn unexpected_change(item: Id) -> Self {
        Self {
            item,
            kind: error::Kind::UnexpectedChange,
        }
    }
}

pub mod error {
    use super::*;

    /// The kind of [Error]
    #[derive(Debug, thiserror::Error)]
    pub enum Kind {
        /// The item does not exist
        #[error("does not exist (expected {expected})")]
        DoesNotExist {
            /// Expected version
            expected: Version,
        },
        /// The item already exist
        #[error("already exists ({actual})")]
        AlreadyExists {
            /// Actual version
            actual: Version,
        },
        /// The item is the wrong version
        #[error("is {actual} (expected {expected})")]
        WrongVersion {
            /// Expected version
            expected: Version,
            /// Actual version
            actual: Version,
        },
        /// The item was expected to change, but did not
        #[error("was not changed as expected")]
        ExpectedChange,
        /// The item was not expected to change, but did
        #[error("was unexpectedly changed")]
        UnexpectedChange,
    }
}
