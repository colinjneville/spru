use std::{any, fmt};

use crate::{game, item::lookup, AnyError, PsuedoError};


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

    /// GameInit is taken by value, so it won't be available once we have the error
    pub(crate) fn prepare_context<GameInit: game::Init>(game_init: &GameInit)
        -> impl FnOnce(Self) -> Self + 'static
    {
        let context = Some(Context::new(game_init));
        |mut e| {
            e.context = context;
            e
        }
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

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Init(AnyError::new(value)))
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
            write!(f, "Game Initializer")?;
        }
        write!(f, " failed: {kind}")?;

        Ok(())
    }
}

impl PsuedoError for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::Lookup(e) => std::error::Error::source(e.as_error()),
            Kind::Init(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub enum Kind {
    Lookup(lookup::Error),
    Init(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Lookup(e) => fmt::Display::fmt(e, f),
            Kind::Init(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Debug)]
struct Context {
    game_init_name: &'static str,
}

impl Context {
    fn new<GameInit: game::Init>(_game_init: &GameInit) -> Self {
        Self {
            game_init_name: any::type_name::<GameInit>(),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            game_init_name,
        } = self;

        write!(f, "Game Initializer '{game_init_name}'")?;

        Ok(())
    }
}