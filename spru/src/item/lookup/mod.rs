pub mod canonical;
pub use canonical::Canonical;
pub mod error;
pub use error::Error;

use std::{any, fmt, ops};

use crate::{item::{self, IdT}, AnyError, Item, PsuedoError};

pub type Result<T> = std::result::Result<T, self::Error>;

pub trait Lookup<T> {
    type Mut<'lr>: ops::DerefMut<Target=Item<T>> + 'lr
    where Self: 'lr;

    fn lookup(&self, id: IdT<T>) -> self::Result<&Item<T>>;
    fn lookup_mut(&mut self, id: IdT<T>) -> self::Result<Self::Mut<'_>>;

    fn exists(&self, id: IdT<T>) -> bool {
        self.lookup(id).is_ok()
    }

    fn create(&mut self, value: Item<T>) -> self::Result<()>;
    fn destroy(&mut self, id: IdT<T>) -> self::Result<Item<T>>;
}
