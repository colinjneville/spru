pub mod canonical;
pub use canonical::Canonical;
pub mod error;
pub use error::Error;

use std::{any, ops};

use crate::{item::IdT, state, Item};

pub type Result<T> = std::result::Result<T, self::Error>;

pub trait Lookup {
    type State: crate::State;

    fn lookup<T>(&self, id: IdT<T>) 
        -> self::Result<&Item<T>>
    where
        T: Lookupable<Self::State>,
        // Self::State: tagset::TagSetDiscriminant<T>,
        // T: any::Any + serde::Serialize + Send + Sync,
    ;
    
    fn lookup_mut<T>(&mut self, id: IdT<T>) 
        -> self::Result<impl ops::DerefMut<Target=Item<T>>>
    where
        T: Lookupable<Self::State>,
        // Self::State: tagset::TagSetDiscriminant<T>,
        // T: any::Any + serde::Serialize + Send + Sync,
    ;

    fn exists<T>(&self, id: IdT<T>) 
        -> bool
    where
        T: Lookupable<Self::State>,
        // Self::State: tagset::TagSetDiscriminant<T>,
        // T: any::Any + serde::Serialize + Send + Sync,
    {
        self.lookup(id).is_ok()
    }

    fn create<T>(&mut self, value: Item<T>) 
        -> self::Result<()>
    where
        T: Lookupable<Self::State>,
        // Self::State: tagset::TagSetDiscriminant<T>,
        // T: any::Any + serde::Serialize + Send + Sync,
    ;
    fn destroy<T>(&mut self, id: IdT<T>) 
        -> self::Result<Item<T>>
    where
        T: Lookupable<Self::State>,
        // Self::State: tagset::TagSetDiscriminant<T>,
        // T: any::Any + serde::Serialize + Send + Sync,
    ;
}

#[implied_bounds::implied_bounds]
pub trait Lookupable<State>: any::Any + serde::Serialize + Sized + Send + Sync
where
    State: tagset::TagSetDiscriminant<Self>,
{ }

impl<State, T> Lookupable<State> for T 
where
    State: tagset::TagSetDiscriminant<T>,
    T: any::Any + serde::Serialize + Sized + Send + Sync,
{ }
