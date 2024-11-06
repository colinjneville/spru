use crate::{action::{self, adapter::Data, Adapter, Output}, item};

pub struct Update;

impl Adapter for Update {
    type In<'l, T, Lookup: item::lookup::OfTypeMut<T>> = item::Mut<<Lookup as item::lookup::OfTypeMut<T>>::Mut<'l>>
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


        let mut value = lookup.lookup_mut(&item::IdT::new(id.clone()))
            .map_err(action::catalog::Error::Lookup)?;
        if version.before == (*value).version() {
            (*value).set_version(version.after);
            Ok(item::Mut::new(value))
        } else {
            Err(action::catalog::Error::Version(item::version::MismatchError { expected: version.before, actual: (*value).version() }))
        }
    }

    fn output<T, Lookup: item::lookup::OfTypeMut<T>, Undo, Error>(_data: &mut Data<Lookup>, output: Output<Undo, Self::Out<T, Lookup>>) 
        -> Result<Option<Undo>, action::catalog::Error<Lookup::Error, Error>> 
    {
        let Output { out: (), undo } = output;
        Ok(undo)
    }
}

// pub struct Updater<'l, Lookup: item::Lookup> {
//     lookup: &'l mut Lookup,
//     id: item::Id,
//     version: version::Change,
// }

// impl<'l, Lookup: item::Lookup> Updater<'l, Lookup> {
//     pub(crate) fn new(lookup: &'l mut Lookup, id: item::Id, version: version::Change) -> Self {
//         Self {
//             lookup,
//             id,
//             version,
//         }
//     }

//     pub fn get<T>(&self) -> Result<&Item<T>, super::LookupVersionError<Lookup::Error>>
//     where Lookup: lookup::OfTypeMut<T>,
//     {
//         let value = self.lookup.lookup(&item::IdT::new(self.id.clone()))
//             .map_err(super::LookupVersionError::Lookup)?;
//         if self.version.before == value.version() {
//             Ok(value)
//         } else {
//             Err(version::MismatchError { expected: self.version.before, actual: value.version() }.into())
//         }
//     }

//     // TODO was T: 'l
//     pub fn get_mut<T>(self) -> Result<Lookup::Mut<'l>, super::LookupVersionError<Lookup::Error>>
//     where 
//         Lookup: lookup::OfTypeMut<T>, 
//     {
//         let mut value = self.lookup.lookup_mut(&item::IdT::new(self.id))
//             .map_err(super::LookupVersionError::Lookup)?;
//         if self.version.before == (*value).version() {
//             (*value).set_version(self.version.after);
//             Ok(value)
//         } else {
//             Err(version::MismatchError { expected: self.version.before, actual: (*value).version() }.into())
//         }
//     }
// }
