use crate::{action::{self, adapter::{Data}, Adapter, Output}, item::{self}};

pub struct Destroy;

impl Adapter for Destroy {
    type In<'l, T, Lookup: item::lookup::OfTypeMut<T>> = T
    where 
        T: 'l,
        Lookup: 'l,
    ;
    type Out<T, Lookup: item::lookup::OfTypeMut<T>> = ();

    fn input<'l, T, Lookup: item::lookup::OfTypeMut<T>, Error>(data: &mut Data<'l, Lookup>) 
        -> Result<Self::In<'l, T, Lookup>, action::catalog::Error<Lookup::Error, Error>>
    where 
        T: 'l,
        Lookup: 'l,
    {
        let Data {
            lookup,
            id, 
            version,
        } = data;
        let lookup = lookup.take().expect("Data is only accessed here");

        let stateful = lookup.lookup(item::IdT::new(id))
            .map_err(action::catalog::Error::Lookup)?;
        if version.before == stateful.version() {
            let stateful = lookup.destroy(item::IdT::new(id))
                .map_err(action::catalog::Error::Lookup)?;
            Ok(stateful.into_value())
        } else {
            Err(action::catalog::Error::Version(item::version::MismatchError { expected: version.before, actual: stateful.version() }))
        }
    }

    fn output<T, Lookup: item::lookup::OfTypeMut<T>, Undo, Error>(_data: &mut Data<Lookup>, output: Output<Undo, Self::Out<T, Lookup>>) 
        -> Result<Option<Undo>, action::catalog::Error<Lookup::Error, Error>>
    {
        let Output { out: (), undo } = output;

        // TODO Ideally we prevent this at compile-time
        undo.as_ref().expect("destroy Action must return an undo record");

        Ok(undo)
    }
}
