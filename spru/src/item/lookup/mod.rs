pub mod canonical;
pub use canonical::Canonical;

use std::ops;

use crate::{Item, item::IdT};

pub trait Lookup {
    type Error;
}

pub trait OfType<T>: Lookup {
    fn lookup(&self, id: IdT<T>) -> Result<&Item<T>, Self::Error>;
}

pub trait OfTypeMut<T>: OfType<T> {    
    type Mut<'lr>: ops::DerefMut<Target=Item<T>> + 'lr
    where Self: 'lr;
    
    fn lookup_mut(&mut self, id: IdT<T>) -> Result<Self::Mut<'_>, Self::Error>;

    fn create(&mut self, value: Item<T>) -> Result<(), Self::Error>;
    fn destroy(&mut self, id: IdT<T>) -> Result<Item<T>, Self::Error>;
}

// pub struct Mut<T, LookupMut>(LookupMut, PhantomData<T>);

// impl<T, LookupMut> Mut<T, LookupMut> {
//     pub(crate) fn new(lookup_mut: LookupMut) -> Self {
//         Self(lookup_mut, PhantomData::default())
//     }
// }

// impl<T, LookupMut> ops::Deref for Mut<T, LookupMut>
// where
//     LookupMut: ops::Deref<Target=Item<T>>,
// {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         self.0.get()
//     }
// }

// impl<T, LookupMut> ops::DerefMut for Mut<T, LookupMut>
// where 
//     LookupMut: ops::DerefMut<Target=Item<T>>,
// {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.0.get_mut()
//     }
// }

