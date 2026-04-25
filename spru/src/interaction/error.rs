use std::{fmt, ops};

use crate::{
    action,
    common::error::{AnyError, PseudoError},
    item::{self, storage},
};

/// An error encountered during an [Interaction](trait@crate::Interaction).
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

    pub(crate) fn with_context<Interaction>(mut self, interaction: &Interaction) -> Self {
        self.context = Some(Context::new(interaction));
        self
    }

    pub(crate) fn new_validation_error(error: item::Error) -> Self {
        Self::new(Kind::Validation(error))
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

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::new(Kind::Action(value))
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Interaction(AnyError::new(value)))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { kind, context } = self;

        if let Some(context) = context {
            write!(f, "{context}")?;
        } else {
            write!(f, "Interaction")?;
        }

        write!(f, " failed: {kind}")?;

        Ok(())
    }
}

impl PseudoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::Storage(e) => std::error::Error::source(e.as_error()),
            Kind::Action(e) => std::error::Error::source(e.as_error()),
            Kind::Validation(e) => std::error::Error::source(e),
            Kind::Interaction(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Context {
    interaction_name: &'static str,
}

impl Context {
    pub(crate) fn new<Interaction>(_interaction: &Interaction) -> Self {
        Self {
            interaction_name: std::any::type_name::<Interaction>(),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { interaction_name } = self;

        write!(f, "Interaction '{interaction_name}'")?;

        Ok(())
    }
}

/// The inner error contained in the [Error]
#[derive(Debug)]
pub enum Kind {
    /// A [storage::Error]
    Storage(storage::Error),
    /// An [action::Error]
    Action(action::Error),
    /// Server-side validation of the [Interaction](trait@crate::Interaction) failed
    Validation(item::Error),
    /// A user-provided error
    Interaction(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(e) => fmt::Display::fmt(e, f),
            Self::Action(e) => fmt::Display::fmt(e, f),
            Self::Validation(e) => fmt::Display::fmt(e, f),
            Self::Interaction(e) => fmt::Display::fmt(e, f),
        }
    }
}
