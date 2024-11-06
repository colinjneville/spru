use crate::{action::{self, adapter}, item};

pub use spru_macro::ActionCatalog as Catalog;

pub trait Catalog<Lookup: item::Lookup>: action::Base {
    fn apply(&self, data: action::adapter::Data<Lookup>) -> Result<Option<Self>, Error<Lookup::Error, Self::Error>>;
}

#[doc(hidden)]
pub trait Entry<Lookup: item::Lookup>: action::Base {
    fn __apply_entry(&self, data: adapter::Data<Lookup>) -> Result<Option<Self::Undo>, action::catalog::Error<Lookup::Error, Self::Error>>
    // where 
    //     Lookup: 'l,
    //     Self::Adapter: action::Adapter,
        ;
}

// A blanket impl like this could technically conflict with specific Action impls
// because of the Lookup type parameter (a downstream crate could define a `struct L`, `impl Lookup for L` and 
// `impl Action<L> for UpstreamCatalog`). Since rust doesn't let us do this, we need to make specific impls
// for Action too in the ActionCatalog macro.
// impl<Lookup: item::Lookup, ActionCatalog: Apply<Lookup>> super::Action<Lookup> for ActionCatalog {
//     type Adapter = adapter::Passthrough;
//     type Error = Error<Lookup::Error, <Self as Catalog>::Error>;

//     type Undo = Self;

//     fn apply(&self, input: <Self::Adapter as action::Adapter<Lookup>>::In<'_>) -> Result<impl Into<action::Output<Self::Undo, <Self::Adapter as action::Adapter<Lookup>>::Out>>, Self::Error> {
//         let out = <ActionCatalog as Apply<Lookup>>::apply(&self, input)?;
//         Ok(out)
//     }
// }

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
