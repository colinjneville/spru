use crate::{action::{self, adapter::Data, Adapter, Output}, item::{self, lookup}, Item};

pub struct Create;

impl Adapter for Create {
    type In<'l, T, Lookup: item::lookup::OfType<T>> = ()
    where 
        T: 'l,
        Lookup: 'l,
    ;
    type Out<T, Lookup: item::lookup::OfType<T>> = T;
    
    fn input<'l, T, Lookup: item::lookup::OfType<T>, Error>(_data: &mut Data<'l, Lookup>) -> Result<(), action::catalog::Error<lookup::Error, Error>>
    where 
        T: 'l,
        Lookup: 'l,
    {
        Ok(())
    }

    fn output<T, Lookup: item::lookup::OfType<T>, Undo, Error>(data: &mut Data<Lookup>, output: Output<Undo, T>) 
        -> Result<Option<Undo>, action::catalog::Error<lookup::Error, Error>>
    {
        let Data { 
            lookup, 
            id, 
            version, 
        } = data;
        let lookup = lookup.take().expect("Data is only accessed here");
        let Output { undo, out } = output;

        if let Ok(stateful) = lookup.lookup(item::IdT::new(id)) {
            Err(action::catalog::Error::Item(item::id::Error::AlreadyExists { id, version: stateful.version() }.into()))
        } else {
            let stateful = Item::new(item::IdT::new(id), version.after, out);
            lookup.create(stateful).map_err(action::catalog::Error::Lookup)?;
            undo.as_ref().expect("create Action must return an undo record");
            Ok(undo)
        }
    }
}

struct Creator<'l, Lookup: item::Lookup> {
    lookup: &'l mut Lookup,
    id: item::Id,
    version: item::version::Change,
}

impl<'l, Lookup: item::Lookup> Creator<'l, Lookup> {
    pub(crate) fn new(lookup: &'l mut Lookup, id: item::Id, version: item::version::Change) -> Self {
        Self {
            lookup,
            id,
            version,
        }
    }

    pub fn create<T>(self, value: T) -> Result<(), Error<lookup::Error>>
    where Lookup: lookup::OfType<T> {
        if let Ok(stateful) = self.lookup.lookup(item::IdT::new(self.id)) {
            Err(Error::Item(item::id::Error::AlreadyExists { id: self.id, version: stateful.version() }.into()))
        } else {
            let stateful = Item::new(item::IdT::new(self.id), self.version.after, value);
            Ok(self.lookup.create(stateful).map_err(Error::Lookup)?)
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError> {
    Lookup(LookupError),
    Item(#[from] item::id::Error),
}
