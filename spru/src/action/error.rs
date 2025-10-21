use std::fmt;

use crate::{item::lookup, AnyError, PsuedoError};


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

    pub(crate) fn with_context<Action: ?Sized>(mut self, action: &Action) -> Self {
        self.context = Some(Context::new(action));
        self
    }
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(Kind::Lookup(value))
    }
}

impl<E: Into<AnyError>> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Action(value.into()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            context,
        } = self;

        if let Some(context) = context{
            write!(f, "{context}")?;
        } else {
            write!(f, "Action")?;
        }
        write!(f, " failed: {kind}")?;

        Ok(())
    }
}

impl PsuedoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::Lookup(e) => std::error::Error::source(e.as_error()),
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
        let Self {
            action_name,
        } = self;

        write!(f, "Action '{action_name}'")?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum Kind {
    /// An error occurred during [crate::Item] []
    Lookup(lookup::Error),
    /// An
    Action(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Lookup(e) => fmt::Display::fmt(e, f),
            Kind::Action(e) => fmt::Display::fmt(e, f),
        }
    }
}

