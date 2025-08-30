use std::fmt;

use crate::{item::{self, lookup}, CustomError};

pub trait Init {
    type Root;
    type State: crate::State<item::lookup::Canonical<Self::State>>;
    type Action: crate::Action<item::lookup::Canonical<Self::State>>;

    fn initialize(self, interactor: &mut Interactor<Self::State, Self::Action>) 
        -> self::Result<Self::Root>;
}

pub type Interactor<'l, State, Action> = crate::Interactor<'l, item::lookup::Canonical<State>, Action, Context, Output>;

#[derive(Debug)]
#[non_exhaustive]
pub struct Context {
    
}

#[derive(Debug)]
#[doc(hidden)]
pub struct Output {
    
}


#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    context: Option<ErrorContext>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub(crate) fn set_context<PlayerInit>(&mut self, _player_init: &PlayerInit) {
        self.context = Some(ErrorContext {
            game_init_name: std::any::type_name::<PlayerInit>(),
        })
    }
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(ErrorKind::Lookup(value))
    }
}

impl<E: std::error::Error + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(ErrorKind::Init(CustomError::new(value)))
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

#[derive(Debug)]
pub enum ErrorKind {
    Lookup(lookup::Error),
    Init(CustomError),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Lookup(e) => fmt::Display::fmt(e, f),
            ErrorKind::Init(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Debug)]
struct ErrorContext {
    game_init_name: &'static str,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            game_init_name,
        } = self;

        write!(f, "Game Initializer '{game_init_name}'")?;

        Ok(())
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;