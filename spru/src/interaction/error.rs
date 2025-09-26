use std::fmt;

use crate::{action, item::lookup, AnyError, PsuedoError};


#[derive(Debug, Default)]
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

    pub(crate) fn set_context<Interaction: crate::Interaction>(&mut self, interaction: &Interaction) {
        self.context = Some(Context::new(interaction))
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(Kind::Lookup(value))
    }
}

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::new(Kind::Record(value))
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Interaction(AnyError::new(value)))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            context,
        } = self;

        if let Some(context) = context {
            write!(f, "{context}")?;
        } else {
            write!(f, "Interaction")?;
        }

        write!(f, " failed: {kind}")?;
        
        Ok(())
    }
}

impl PsuedoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::Lookup(e) => std::error::Error::source(e.as_error()),
            Kind::Record(e) => std::error::Error::source(e.as_error()),
            Kind::Interaction(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Context {
    interaction_name: &'static str,
}

impl Context {
    pub(crate) fn new<Interaction: crate::Interaction>(_interaction: &Interaction) -> Self {
        Self {
            interaction_name: std::any::type_name::<Interaction>(),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            interaction_name,
        } = self;

        write!(f, "Interaction '{interaction_name}'")?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum Kind {
    Lookup(lookup::Error),
    Record(action::Error),
    Interaction(AnyError),
}

impl Default for Kind {
    fn default() -> Self {
        Self::Interaction(AnyError::default())
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lookup(e) => fmt::Display::fmt(e, f),
            Self::Record(e) => fmt::Display::fmt(e, f),
            Self::Interaction(e) => fmt::Display::fmt(e, f),
        }
    }
}