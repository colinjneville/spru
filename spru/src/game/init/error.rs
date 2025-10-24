use std::{any, fmt, ops};

use crate::{action, common::error::{AnyError, PsuedoError}, game, item::lookup};


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
        -> impl FnMut(Self) -> Self + 'static
    {
        let context = Some(Context::new(game_init));
        move|mut e| {
            e.context = context.clone();
            e
        }
    }

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

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(Kind::Action(action::Error::from(value)))
    }
}

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::new(Kind::Action(value))
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
            Kind::Action(e) => std::error::Error::source(e.as_error()),
            Kind::Init(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub enum Kind {
    Action(action::Error),
    Init(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Action(e) => fmt::Display::fmt(e, f),
            Kind::Init(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Debug, Clone)]
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