use std::fmt;

use crate::{item::{self, lookup}, record, CustomError, Item};

pub use spru_macro::{Create, Update, Destroy};
use tagset::tagset_meta;

#[telety::telety(crate::action)]
#[tagset_meta]
pub trait Base {
    #[meta(default(Self))]
    type Undo;
}

#[telety::telety(crate::action, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(
    for<VAR> VAR: 
        crate::Action<
            Lookup,
            Undo: Into<Self::Undo>, 
            Error: Into<Self::Error>
        >,
))]
pub trait Action<Lookup>: Base {
    #[meta(default {
        match_by_value!(self, v => spru::Action::apply_map(v, context))
    })]
    fn apply(&self, context: Context<'_, Lookup>) -> record::Result<Option<Self::Undo>>
    where 
        Self: Sized;

    fn apply_map<U>(&self, context: Context<'_, Lookup>) -> record::Result<Option<U>>
    where 
        Self: Sized,
        Self::Undo: Into<U>,
    {
        self.apply(context)
            .map(|u| u.map(Into::into))
    }
}

pub trait Create {
    type T;
    type Undo;

    fn create(&self) -> self::Result<(Self::T, Self::Undo)>;
}

pub trait Update {
    type T;
    type Undo;
    type Return<'t>;

    fn update<'t>(&self, value: &'t mut Self::T) 
        -> self::Result<impl Into<UpdateReturn<Self::Undo, Self::Return<'t>>>>;
}

pub struct UpdateReturn<Undo, Return> {
    pub(crate) undo: Option<Undo>,
    pub(crate) return_value: Return,
}

impl<Undo, Return> UpdateReturn<Undo, Return> {
    fn map_undo<U>(self) -> Option<U> 
    where Undo: Into<U> {
        self.undo.map(Into::into)
    }
}

impl<Undo, Return> From<(Undo, Return)> for UpdateReturn<Undo, Return> {
    fn from((undo, return_value): (Undo, Return)) -> Self {
        Self {
            undo: Some(undo),
            return_value,
        }
    }
}

impl<Undo, Return> From<(Option<Undo>, Return)> for UpdateReturn<Undo, Return> {
    fn from((undo, return_value): (Option<Undo>, Return)) -> Self {
        Self {
            undo,
            return_value,
        }
    }
}

impl<Undo, Return> From<Return> for UpdateReturn<Undo, Return> {
    fn from(return_value: Return) -> Self {
        Self {
            undo: None,
            return_value,
        }
    }
}

pub trait Destroy {
    type T;
    type Undo;

    fn destroy(&self, value: Self::T) -> self::Result<Self::Undo>;
}

#[derive(Debug)]
pub struct Context<'l, Lookup> {
    pub(crate) lookup: &'l mut Lookup,
    pub(crate) id: item::Id,
    pub(crate) version: item::version::Change,
}

impl<'l, Lookup> Context<'l, Lookup> {
    pub(crate) fn new(lookup: &'l mut Lookup, id: item::Id, version: item::version::Change) -> Self {
        Self {
            lookup,
            id,
            version,
        }
    }

    #[doc(hidden)]
    pub fn create<C, O>(self, c: &C) -> record::Result<Option<O>> 
    where 
        Lookup: item::lookup::Lookup<C::T>,
        C: Create,
        C::Undo: Into<O>,
    {
        let Self { 
            lookup, 
            id, 
            version, 
        } = self;

        if let Ok(stateful) = lookup.lookup(item::IdT::new(id)) {
            Err(record::Error::Item(item::id::Error::AlreadyExists { id: id.clone(), version: stateful.version() }.into()))
        } else {
            let (value, undo) = c.create()?;

            let stateful = Item::new(item::IdT::new(id.clone()), version.after, value);
            lookup.create(stateful)?;

            Ok(Some(undo.into()))
        }
    }

    #[doc(hidden)]
    pub fn update<U, O>(self, u: &U) -> record::Result<Option<O>> 
    where 
        Lookup: item::lookup::Lookup<U::T>,
        U: Update,
        U::Undo: Into<O>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let mut value = lookup.lookup_mut(item::IdT::new(id))?;
        if version.before == (*value).version() {
            (*value).set_version(version.after);
            u.update(value.get_mut())
                .map(Into::into)
                .map(UpdateReturn::map_undo)
                .map_err(Into::into)
        } else {
            Err(record::Error::Version(item::version::Error { expected: version.before, actual: (*value).version() }))
        }
    }

    #[doc(hidden)]
    pub fn destroy<D, O>(self, d: &D) -> record::Result<Option<O>> 
    where 
        Lookup: item::lookup::Lookup<D::T>,
        D: Destroy,
        D::Undo: Into<O>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let item = lookup.lookup(item::IdT::new(id))?;
        if version.before == item.version() {
            let item = lookup.destroy(item::IdT::new(id))?;
            let undo = d.destroy(item.into_value())?;
            Ok(Some(undo.into()))
        } else {
            Err(record::Error::Version(item::version::Error { expected: version.before, actual: item.version() }))
        }
    }
}

#[derive(Debug)]
pub struct Error<const ImplError: bool = false> {
    kind: ErrorKind,
    context: Option<ErrorContext>,
}

impl<const ImplError: bool = false> Error<ImplError> {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub(crate) fn with_context<Action>(mut self, action: &Action) -> Self {
        self.context = Some(ErrorContext::new(action));
        self
    }
}

impl Error<false> {
    pub fn into_error(self) -> Error<true> {
        
    }
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::new(ErrorKind::Lookup(value))
    }
}

impl<E: std::error::Error + 'static> From<E> for Error {
    fn from(value: E) -> Self {
        Self::new(ErrorKind::Action(CustomError::new(value)))
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

#[derive(Debug)]
pub(crate) struct ErrorContext {
    action_name: &'static str,
}

impl ErrorContext {
    pub(crate) fn new<Action>(_action: &Action) -> Self {
        Self {
            action_name: std::any::type_name::<Action>(),
        }
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            action_name,
        } = self;

        write!(f, "Action '{action_name}'")?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Lookup(lookup::Error),
    Action(CustomError),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Lookup(e) => fmt::Display::fmt(e, f),
            ErrorKind::Action(e) => fmt::Display::fmt(e, f),
        }
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;
