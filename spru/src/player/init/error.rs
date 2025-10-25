use std::{any, fmt, ops};

use crate::{
    action,
    common::error::{AnyError, PsuedoError},
    item, player,
};

#[derive(Debug)]
pub struct Error {
    kind: Kind,
    context: Option<Context>,
}

impl ops::Deref for Error {
    type Target = dyn std::error::Error;

    fn deref(&self) -> &Self::Target {
        self.as_error()
    }
}

impl Error {
    pub(crate) fn new(kind: Kind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub(crate) fn with_context<PlayerInit: player::Init>(
        mut self,
        player_init: &PlayerInit,
    ) -> Self {
        self.context = Some(Context::new(player_init));
        self
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::new(Kind::Record(value))
    }
}

impl From<item::lookup::Error> for Error {
    fn from(value: item::lookup::Error) -> Self {
        Self::new(Kind::Record(value.into()))
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(Kind::Init(AnyError::new(value)))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { kind, context } = self;

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
            Kind::Record(e) => std::error::Error::source(e.as_error()),
            Kind::Init(e) => std::error::Error::source(e.as_error()),
        }
    }
}

#[derive(Debug)]
pub enum Kind {
    Record(action::Error),
    Init(AnyError),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Record(e) => fmt::Display::fmt(e, f),
            Kind::Init(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Debug)]
struct Context {
    player_init_name: &'static str,
}

impl Context {
    fn new<PlayerInit: player::Init>(_game_init: &PlayerInit) -> Self {
        Self {
            player_init_name: any::type_name::<PlayerInit>(),
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { player_init_name } = self;

        write!(f, "Player Initializer '{player_init_name}'")?;

        Ok(())
    }
}
