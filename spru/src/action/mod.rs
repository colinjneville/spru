pub mod error;
pub use error::Error;

use crate::{action, error::AnyResult, item, Item};

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
        >,
))]
pub trait Action<Lookup>: Base {
    #[meta(default {
        match_by_value!(self, v => spru::Action::apply_map(v, context))
    })]
    fn apply(&self, context: Context<'_, Lookup>) -> action::Result<Option<Self::Undo>>
    where 
        Self: Sized;

    fn apply_map<U>(&self, context: Context<'_, Lookup>) -> action::Result<Option<U>>
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

    fn create(&self) -> AnyResult<(Self::T, Self::Undo)>;
}

pub trait Update {
    type T;
    type Undo;

    fn update(&self, value: &mut Self::T) 
        -> AnyResult<impl Into<Option<Self::Undo>>>;
}

pub trait UpdateReturn: Update {
    type Return;

    fn return_value(&self, value: &Self::T) 
        -> AnyResult<Self::Return>;
}

pub trait Destroy {
    type T;
    type Undo;

    fn destroy(&self, value: Self::T) -> AnyResult<Self::Undo>;
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
    pub fn create<C, O>(self, c: &C) -> action::Result<Option<O>> 
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
            Err(action::Error::from(item::id::Error::AlreadyExists { id: id.clone(), version: stateful.version() }))
        } else {
            let (value, undo) = c.create()?;

            let stateful = Item::new(item::IdT::new(id.clone()), version.after, value);
            lookup.create(stateful)?;

            Ok(Some(undo.into()))
        }
    }

    #[doc(hidden)]
    pub fn update<U, O>(self, u: &U) -> action::Result<Option<O>> 
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
                .map(|o| o.map(Into::into))
                .map_err(Into::into)
        } else {
            Err(action::Error::from(item::version::Error { expected: version.before, actual: (*value).version() }))
        }
    }

    #[doc(hidden)]
    pub fn destroy<D, O>(self, d: &D) -> action::Result<Option<O>> 
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
            Err(action::Error::from(item::version::Error { expected: version.before, actual: item.version() }))
        }
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;
