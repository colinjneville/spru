pub mod create;
pub use create::Create;
pub mod destroy;
pub use destroy::Destroy;
pub mod update;
pub use update::Update;

use crate::{action::{self, Output}, item};

#[derive(Debug)]
pub struct Data<'l, Lookup> {
    pub(crate) lookup: Option<&'l mut Lookup>,
    pub(crate) id: item::Id,
    pub(crate) version: item::version::Change,
}

impl<'l, Lookup> Data<'l, Lookup> {
    pub(crate) fn new(lookup: &'l mut Lookup, id: item::Id, version: item::version::Change) -> Self {
        Self {
            lookup: Some(lookup),
            id,
            version,
        }
    }
}

pub trait Adapter {
    type In<'l, T, Lookup: item::lookup::OfTypeMut<T>>
    where 
        T: 'l,
        Lookup: 'l,
    ;
    type Out<T, Lookup: item::lookup::OfTypeMut<T>>;

    fn input<'l, T, Lookup: item::lookup::OfTypeMut<T>, Error>(data: &mut Data<'l, Lookup>) 
        -> Result<Self::In<'l, T, Lookup>, action::catalog::Error<Lookup::Error, Error>>
    where 
        T: 'l,
        Lookup: 'l,
    ;
    fn output<T, Lookup: item::lookup::OfTypeMut<T>, Undo, Error>(data: &mut Data<Lookup>, output: Output<Undo, Self::Out<T, Lookup>>) 
        -> Result<Option<Undo>, action::catalog::Error<Lookup::Error, Error>>;
}

// TODO rename
#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum LookupVersionError<LookupError> {
    #[error(transparent)]
    Lookup(LookupError),
    Version(#[from] item::version::MismatchError),
}
