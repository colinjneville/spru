use std::{any, fmt};

use crate::{item::{self, IdT}, AnyError, PsuedoError};

#[derive(Debug, Default)]
pub struct Error {
    inner: AnyError,
    context: Option<Context>,
}

impl Error {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self {
            inner: AnyError::new(error),
            context: None,
        }
    }

    pub(crate) fn with_context<T>(mut self, id: IdT<T>) -> Self {
        self.context = Some(Context {
            id: id.untyped(),
            type_name: any::type_name::<T>(),
        });
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            inner,
            context,
        } = self;

        write!(f, "Item ")?;
        if let Some(context) = context {
            write!(f, " {context}")?;
        }
        write!(f, "not found: {inner}")?;

        Ok(())
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl PsuedoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(self.inner.as_error())
    }
}

#[derive(Debug)]
pub struct Context {
    id: item::Id,
    type_name: &'static str,
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            id,
            type_name,
        } = self;
        
        write!(f, "{id} ({type_name})")?;
        Ok(())
    }
}