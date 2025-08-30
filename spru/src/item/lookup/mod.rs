pub mod canonical;
pub use canonical::Canonical;

use std::{any, fmt, ops};

use crate::{item::{self, IdT}, Item};

pub trait Lookup<T> {
    type Mut<'lr>: ops::DerefMut<Target=Item<T>> + 'lr
    where Self: 'lr;

    fn lookup(&self, id: IdT<T>) -> Result<&Item<T>, Error>;
    fn lookup_mut(&mut self, id: IdT<T>) -> Result<Self::Mut<'_>, Error>;

    fn exists(&self, id: IdT<T>) -> bool {
        self.lookup(id).is_ok()
    }

    fn create(&mut self, value: Item<T>) -> Result<(), Error>;
    fn destroy(&mut self, id: IdT<T>) -> Result<Item<T>, Error>;
}

#[derive(Debug, Default)]
pub struct Error {
    inner: Option<Box<dyn std::error::Error>>,
    id: Option<item::Id>,
    type_name: Option<&'static str>,
}

impl Error {
    pub fn new<E: std::error::Error + 'static>(error: E) -> Self {
        Self {
            inner: Some(Box::new(error)),
            id: None,
            type_name: None,
        }
    }

    pub(crate) fn set_id<T>(&mut self, id: IdT<T>) {
        self.id = Some(id.untyped());
        self.type_name = Some(any::type_name::<T>());
    }

    pub fn try_cast<E: std::error::Error + 'static>(self) -> Result<E, Self> {
        let Self {
            mut inner,
            id,
            type_name,
        } = self;

        if let Some(some_inner) = inner {
            match some_inner.downcast() {
                Ok(e) => return Ok(*e),
                Err(e) => inner = Some(e),
            }
        }

        Err(Self {
            inner,
            id,
            type_name,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Item ")?;
        if let Some(id) = &self.id {
            write!(f, "{id} ")?;
        }
        if let Some(type_name)  = &self.type_name {
            write!(f, "({type_name}) ")?;
        }
        write!(f, "not found")?;

        if let Some(inner) = &self.inner {
            write!(f, ": {inner}")?;
        }
        Ok(())
    }
}

impl<E: std::error::Error + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}
