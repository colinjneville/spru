//! Errors encountered while applying [Action](trait@super::Action)s.

use std::{fmt, ops};

use crate::{
    common::error::{AnyError, PseudoError},
    item::{self, storage},
};

/// An error encountered while applying an [trait@super::Action].
/// [std::ops::Deref] to use as a [std::error::Error].
#[derive(Debug)]
pub struct Error {
    kind: Kind,
    context: Option<Context>,
}

impl Error {
    pub(crate) fn new(kind: Kind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub(crate) fn new_item(item_error: item::Error) -> Self {
        Self::new(Kind::Item(item_error))
    }

    pub(crate) fn with_context<Action: ?Sized>(mut self, action: &Action) -> Self {
        self.context = Some(Context::new(action));
        self
    }

    /// The contained inner error
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

impl ops::Deref for Error {
    type Target = dyn std::error::Error;

    fn deref(&self) -> &Self::Target {
        self.as_error()
    }
}

impl From<storage::Error> for Error {
    fn from(value: storage::Error) -> Self {
        Self::new(Kind::Storage(value))
    }
}

impl<E: Into<AnyError>> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Action(value.into()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { kind, context } = self;

        if let Some(context) = context {
            write!(f, "{context}")?;
        } else {
            write!(f, "Action")?;
        }
        write!(f, " failed: {kind}")?;

        Ok(())
    }
}

impl PseudoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::Storage(e) => std::error::Error::source(e.as_error()),
            Kind::Item(e) => std::error::Error::source(e),
            Kind::Action(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Context {
    action_name: &'static str,
}

impl Context {
    pub(crate) fn new<Action: ?Sized>(_action: &Action) -> Self {
        Self {
            action_name: std::any::type_name::<Action>(),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { action_name } = self;

        write!(f, "Action '{action_name}'")?;

        Ok(())
    }
}

/// The inner error contained in the [Error]
#[derive(Debug)]
pub enum Kind {
    /// A [storage::Error]
    Storage(storage::Error),
    /// An [item::Error]
    Item(item::Error),
    /// A user-provided error
    Action(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Storage(e) => fmt::Display::fmt(e, f),
            Kind::Item(e) => fmt::Display::fmt(e, f),
            Kind::Action(e) => fmt::Display::fmt(e, f),
        }
    }
}
