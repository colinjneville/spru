use crate::{item, Item};

pub use spru_macro::{Create, Update, Destroy};
use tagset::tagset_meta;

#[telety::telety(crate::action)]
#[tagset_meta]
pub trait Base {
    #[meta(default(Self))]
    type Undo;
    #[meta(default(std::convert::Infallible))]
    type Error;
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
pub trait Action<Lookup: item::Lookup>: Base {
    #[meta(default {
        match_by_value!(self, v => spru::Action::apply_map(v, context))
    })]
    fn apply(&self, context: Context<'_, Lookup>) -> Result<Option<Self::Undo>, Error<Lookup::Error, Self::Error>>
    where 
        Self: Sized;

    fn apply_map<U, E>(&self, context: Context<'_, Lookup>) -> Result<Option<U>, Error<Lookup::Error, E>>
    where 
        Self: Sized,
        Self::Undo: Into<U>,
        Self::Error: Into<E>
    {
        self.apply(context)
            .map(|u| u.map(Into::into))
            .map_err(Error::map_action)
    }
}

pub trait Create {
    type T;
    type Undo;
    type Error;

    fn create(&self) -> Result<(Self::T, Self::Undo), Self::Error>;
}

pub trait Update {
    type T;
    type Undo;
    type Error;
    type Return<'t>;

    fn update<'t>(&self, value: &'t mut Self::T) 
        -> Result<impl Into<UpdateReturn<Self::Undo, Self::Return<'t>>>, Self::Error>;
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
    type Error;

    fn destroy(&self, value: Self::T) -> Result<Self::Undo, Self::Error>;
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
    pub fn create<C, O, E>(self, c: &C) -> Result<Option<O>, Error<Lookup::Error, E>> 
    where 
        Lookup: item::lookup::OfTypeMut<C::T>,
        C: Create,
        C::Undo: Into<O>,
        C::Error: Into<E>,
    {
        let Self { 
            lookup, 
            id, 
            version, 
        } = self;

        if let Ok(stateful) = lookup.lookup(item::IdT::new(id)) {
            Err(Error::Item(item::id::Error::AlreadyExists { id: id.clone(), version: stateful.version() }.into()))
        } else {
            let (value, undo) = c.create()
                .map_err(|e| Error::Action(e.into()))?;

            let stateful = Item::new(item::IdT::new(id.clone()), version.after, value);
            lookup.create(stateful).map_err(Error::Lookup)?;

            Ok(Some(undo.into()))
        }
    }

    #[doc(hidden)]
    pub fn update<U, O, E>(self, u: &U) -> Result<Option<O>, Error<Lookup::Error, E>> 
    where 
        Lookup: item::lookup::OfTypeMut<U::T>,
        U: Update,
        U::Undo: Into<O>,
        U::Error: Into<E>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let mut value = lookup.lookup_mut(item::IdT::new(id))
            .map_err(Error::Lookup)?;
        if version.before == (*value).version() {
            (*value).set_version(version.after);
            u.update(value.get_mut())
                .map(Into::into)
                .map(UpdateReturn::map_undo)
                .map_err(|e| Error::Action(e.into()))
        } else {
            Err(Error::Version(item::version::MismatchError { expected: version.before, actual: (*value).version() }))
        }
    }

    #[doc(hidden)]
    pub fn destroy<D, O, E>(self, d: &D) -> Result<Option<O>, Error<Lookup::Error, E>> 
    where 
        Lookup: item::lookup::OfTypeMut<D::T>,
        D: Destroy,
        D::Undo: Into<O>,
        D::Error: Into<E>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let item = lookup.lookup(item::IdT::new(id))
            .map_err(Error::Lookup)?;
        if version.before == item.version() {
            let item = lookup.destroy(item::IdT::new(id))
                .map_err(Error::Lookup)?;
            let undo = d.destroy(item.into_value())
                .map_err(|e| Error::Action(e.into()))?;
            Ok(Some(undo.into()))
        } else {
            Err(Error::Version(item::version::MismatchError { expected: version.before, actual: item.version() }))
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, ActionError> {
    Lookup(LookupError),
    Item(item::id::Error),
    Version(item::version::MismatchError),
    Action(#[from] ActionError)
}

impl<LookupError, ActionError> Error<LookupError, ActionError> {
    #[doc(hidden)]
    pub fn map_action<ActionError2>(self) -> Error<LookupError, ActionError2> 
    where
        ActionError: Into<ActionError2>,
    {
        match self {
            Error::Lookup(err) => Error::Lookup(err),
            Error::Item(err) => Error::Item(err),
            Error::Version(err) => Error::Version(err),
            Error::Action(err) => Error::Action(err.into()),
        }
    }
}
