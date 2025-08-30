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

    #[doc(hidden)]
    pub fn create<T, Undo, Error>(&mut self, value: T, undo: Undo) 
    -> Result<Option<Undo>, action::catalog::Error<lookup::Error, Error>> 
    where
        Lookup: item::lookup::OfType<T>
    {
        let Data { 
            lookup, 
            id, 
            version, 
        } = data;
        let lookup = lookup.take().expect("Data is only accessed here");
        let Output { undo, out } = output;

        if let Ok(stateful) = lookup.lookup(&item::IdT::new(id.clone())) {
            Err(action::catalog::Error::Item(item::id::Error::AlreadyExists { id: id.clone(), version: stateful.version() }.into()))
        } else {
            let stateful = Item::new(item::IdT::new(id.clone()), version.after, out);
            lookup.create(stateful).map_err(action::catalog::Error::Lookup)?;
            undo.as_ref().expect("create Action must return an undo record");
            Ok(undo)
        }
    }
}

pub trait Adapter {
    type In<'l, T, Lookup: item::lookup::OfType<T>>
    where 
        T: 'l,
        Lookup: 'l,
    ;
    type Out<T, Lookup: item::lookup::OfType<T>>;

    fn input<'l, T, Lookup: item::lookup::OfType<T>, Error>(data: &mut Data<'l, Lookup>) 
        -> Result<Self::In<'l, T, Lookup>, action::catalog::Error<lookup::Error, Error>>
    where 
        T: 'l,
        Lookup: 'l,
    ;
    fn output<T, Lookup: item::lookup::OfType<T>, Undo, Error>(data: &mut Data<Lookup>, output: Output<Undo, Self::Out<T, Lookup>>) 
        -> Result<Option<Undo>, action::catalog::Error<lookup::Error, Error>>;
}

// TODO rename
#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum LookupVersionError<LookupError> {
    #[error(transparent)]
    Lookup(LookupError),
    Version(#[from] item::version::MismatchError),
}
