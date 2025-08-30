use std::{collections::VecDeque, fmt};

use derive_where::derive_where;
use tagset::tagset_meta;
use telety::telety;

use crate::{action, interactor, item::{self, lookup}, player, record, CustomError};

#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    pub root: &'r Root,
    pub player: player::Id,
}

#[derive(Debug)]
#[derive_where(Default)]
#[doc(hidden)]
pub struct Output<Trigger> {
    pub(crate) triggers: VecDeque<Trigger>,
}

impl<Trigger> interactor::EnqueueTrigger for Output<Trigger> {
    type Trigger = Trigger;

    fn enqueue_trigger(&mut self, trigger: Self::Trigger) {
        self.triggers.push_back(trigger);
    }
}


impl<'r, Root> Context<'r, Root> {
    pub(crate) fn new(root: &'r Root, player: player::Id) -> Self {
        Self {
            root,
            player,
        }
    }    
}


#[telety(crate::interaction, alias_traits = "always")]
#[tagset_meta]
pub trait Interaction {
    type Action;
    type Root;
    type Trigger;

    fn apply<'l, 'r, Lookup>(&self, interactor: &mut Interactor<'l, 'r, Lookup, Self::Action, Self::Root, Self::Trigger>)
         -> self::Result<()>
    where 
        Self::Action: crate::Action<Lookup>,
    ;
}

pub type Interactor<'l, 'r, Lookup, Action, Root, Trigger> = crate::Interactor<'l, Lookup, Action, Context<'r, Root>, Output<Trigger>>;

#[derive(Debug, Default)]
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
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(ErrorKind::Lookup(value))
    }
}

impl From<record::Error> for Error {
    fn from(value: record::Error) -> Self {
        Self::new(ErrorKind::Record(value))
    }
}

impl<E: std::error::Error + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(ErrorKind::Interaction(CustomError::new(value)))
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

#[derive(Debug)]
pub(crate) struct ErrorContext {
    interaction_name: &'static str,
}

impl ErrorContext {
    pub(crate) fn new<Interaction>(_interaction: &Interaction) -> Self {
        Self {
            interaction_name: std::any::type_name::<Interaction>(),
        }
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            interaction_name,
        } = self;

        write!(f, "Interaction '{interaction_name}'")?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Lookup(lookup::Error),
    Record(record::Error),
    Interaction(CustomError),
}

impl Default for ErrorKind {
    fn default() -> Self {
        Self::Interaction(CustomError::default())
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lookup(e) => fmt::Display::fmt(e, f),
            Self::Record(e) => fmt::Display::fmt(e, f),
            Self::Interaction(e) => fmt::Display::fmt(e, f),
        }
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Staged<Interaction> {
    pub(crate) interaction: Interaction,
    pub(crate) expected_versions: item::version::Expected,
}
