pub mod error;
pub use error::Error;

use crate::{action, error::AnyResult, item, Item};

pub use spru_macro::{Create, Update, Destroy};
use tagset::tagset_meta;

/// Implemented automatically by the Create/Update/Destroy macros
#[doc(hidden)]
pub trait SubAction {
    type Undo;
    type T;

    fn apply<Lookup>(&self, context: Context<'_, Lookup>)
        -> action::Result<Option<Self::Undo>>
    where 
        Lookup: item::Lookup,
        Self::T: item::lookup::Lookupable<Lookup::State>,
    ;

    fn apply_map<Lookup, Action>(&self, context: Context<'_, Lookup>)
        -> action::Result<Option<Action>>
    where 
        Lookup: item::Lookup,
        Self::T: item::lookup::Lookupable<Lookup::State>,
        Self::Undo: Into<Action>,
    {
        self.apply(context)
            .map(|o| o.map(Into::into))
    }
}

#[telety::telety(crate::action, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(
    for<VAR> VAR: 
        crate::action::SubAction<Undo: Into<Self>>,
))]
pub trait Action: Sized {
    type State: crate::State;

    #[meta(default {
        match_by_value!(self, v => spru::action::SubAction::apply_map(v, context))
    })]
    fn apply<Lookup: item::Lookup<State = Self::State>>(&self, context: Context<'_, Lookup>) -> action::Result<Option<Self>>;
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
    pub fn create<C>(self, c: &C) -> action::Result<Option<C::Undo>> 
    where 
        Lookup: item::Lookup,
        C: Create<T: item::lookup::Lookupable<Lookup::State>>,
    {
        let Self { 
            lookup, 
            id, 
            version, 
        } = self;

        if let Ok(stateful) = lookup.lookup(item::IdT::<C::T>::new(id)) {
            Err(action::Error::from(item::id::Error::AlreadyExists { id: id.clone(), version: stateful.version() }))
        } else {
            let (value, undo) = c.create()?;

            let stateful = Item::new(item::IdT::new(id.clone()), version.after, value);
            lookup.create(stateful)?;

            Ok(Some(undo))
        }
    }

    #[doc(hidden)]
    pub fn update<U>(self, u: &U) -> action::Result<Option<U::Undo>> 
    where 
        Lookup: item::lookup::Lookup,
        U: Update<T: item::lookup::Lookupable<Lookup::State>>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let mut value = lookup.lookup_mut(item::IdT::<U::T>::new(id))?;
        if version.before == (*value).version() {
            (*value).set_version(version.after);
            u.update(value.get_mut())
                .map(Into::into)
                .map_err(Into::into)
        } else {
            Err(action::Error::from(item::version::Error { expected: version.before, actual: (*value).version() }))
        }
    }

    #[doc(hidden)]
    pub fn destroy<D>(self, d: &D) -> action::Result<Option<D::Undo>> 
    where 
        Lookup: item::lookup::Lookup,
        D: Destroy<T: item::lookup::Lookupable<Lookup::State>>,
    {
        let Self {
            lookup, 
            id, 
            version,
        } = self;

        let item = lookup.lookup(item::IdT::<D::T>::new(id))?;
        if version.before == item.version() {
            let item = lookup.destroy(item::IdT::<D::T>::new(id))?;
            let undo = d.destroy(item.into_value())?;
            Ok(Some(undo))
        } else {
            Err(action::Error::from(item::version::Error { expected: version.before, actual: item.version() }))
        }
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;
