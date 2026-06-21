pub(crate) mod canonical;
pub use canonical::Canonical;
pub mod error;
pub use error::Error;

use std::{any, ops};

use crate::{Item, item::IdT};

/// A result with an [Error] `Err`
pub type Result<T> = std::result::Result<T, self::Error>;

/// Retrieves [Item]s created by spru.  
///
/// A [Server](crate::Server) provides its own Storage, but a [Client](crate::Client) must
/// choose an implementation.
pub trait ReadOnlyStorage {
    /// The type of [State](trait@crate::State) stored, usually defined by type parameter
    type State: crate::State;

    /// Retrieve an immutable [`Item<T>`] with the given id.  
    ///
    /// Returns an error if the [Item] does not exist.
    fn get<T>(&self, id: IdT<T>) -> self::Result<&Item<T>>
    where
        T: Storable<Self::State>;
}

/// Stores and retrieves [Item]s created by spru.  
///
/// A [Server](crate::Server) provides its own Storage, but a [Client](crate::Client) must
/// choose an implementation.
pub trait Storage: ReadOnlyStorage {
    /// Retrieve a mutable [`Item<T>`] with the given id.  
    ///
    /// Returns an error if the [Item] does not exist.
    fn get_mut<T>(&mut self, id: IdT<T>) -> self::Result<impl ops::DerefMut<Target = Item<T>>>
    where
        T: Storable<Self::State>;

    /// Stores a new [`Item<T>`].  
    ///
    /// Once created, the Storage may not alter the [Item]. (It may still move it, (de)serialize it, etc.)
    fn create<T>(&mut self, value: Item<T>) -> self::Result<()>
    where
        T: Storable<Self::State>;

    /// Removes an [`Item<T>`].
    fn destroy<T>(&mut self, id: IdT<T>) -> self::Result<Item<T>>
    where
        T: Storable<Self::State>;
}

/// Types which can be stored in a [Storage]
#[implied_bounds::implied_bounds]
pub trait Storable<State>: any::Any + serde::Serialize + Sized + Send + Sync
where
    State: tagset::TagSetDiscriminant<Self>,
{
}

impl<State, T> Storable<State> for T
where
    State: tagset::TagSetDiscriminant<T>,
    T: any::Any + serde::Serialize + Sized + Send + Sync,
{
}
