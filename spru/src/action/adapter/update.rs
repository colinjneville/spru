use crate::{action::{self, adapter::Data, Adapter, Output}, item};

pub struct Update;

impl Adapter for Update {
    type In<'l, T, Lookup: item::lookup::OfType<T>> = item::Mut<<Lookup as item::lookup::OfType<T>>::Mut<'l>>
    where 
        T: 'l,
        Lookup: 'l,
    ;
    type Out<T, Lookup: item::lookup::OfType<T>> = ();

    fn input<'l, T, Lookup: item::lookup::OfType<T>, Error>(data: &mut Data<'l, Lookup>) 
        -> Result<Self::In<'l, T, Lookup>, action::catalog::Error<lookup::Error, Error>>
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


        let mut value = lookup.lookup_mut(item::IdT::new(id))
            .map_err(action::catalog::Error::Lookup)?;
        if version.before == (*value).version() {
            (*value).set_version(version.after);
            Ok(item::Mut::new(value))
        } else {
            Err(action::catalog::Error::Version(item::version::MismatchError { expected: version.before, actual: (*value).version() }))
        }
    }

    fn output<T, Lookup: item::lookup::OfType<T>, Undo, Error>(_data: &mut Data<Lookup>, output: Output<Undo, Self::Out<T, Lookup>>) 
        -> Result<Option<Undo>, action::catalog::Error<lookup::Error, Error>> 
    {
        let Output { out: (), undo } = output;
        Ok(undo)
    }
}
