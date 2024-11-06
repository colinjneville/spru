pub use spru_macro::ItemCatalog as Catalog;

use crate::{snapshot, item, Item};

pub trait Catalog<Lookup>
where 
    Lookup: item::Lookup,
{
    #[doc(hidden)]
    fn apply_item(index: Index, data: &Item<Box<[u8]>>, lookup: &mut Lookup) -> Result<(), snapshot::ApplyError<Lookup::Error>>;
}

pub type Index = u32;


#[doc(hidden)]
pub fn do_apply_item<Lookup, T>(item: &Item<Box<[u8]>>, lookup: &mut Lookup) -> Result<(), snapshot::ApplyError<Lookup::Error>> 
where 
    Lookup: item::lookup::OfTypeMut<T>,
    T: serde::de::DeserializeOwned,
{
    let id = item::IdT::new(*item.id().untyped());
    let version = item.version();
    let value = rmp_serde::from_slice::<T>(&*item.get())?;
    let item = Item::new(id, version, value);
    lookup.create(item)
        .map_err(snapshot::ApplyError::Lookup)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    extern crate self as spru;

    #[test]
    fn derive() {
        #[derive(Catalog)]
        #[repr(u32)]
        enum MyCatalog {
            A(u8) = 1,
            B(u16),
            C(u32) = 7,
            D(u64),
        }
    }
}