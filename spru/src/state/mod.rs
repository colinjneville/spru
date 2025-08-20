use tagset::tagset_meta;

use crate::{snapshot, item, Item};

pub type Index = u32;

#[telety::telety(crate::state, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(for<VAR> Lookup: item::lookup::OfTypeMut<VAR>))]
pub trait State<Lookup>: tagset::TagSet<Repr: Into<Index>>
where 
    Lookup: item::Lookup,
{
    #[doc(hidden)]
    #[meta(default {
        match_by_discriminant!(index, T => Self::do_apply_state::<T>(item, lookup))
    })]
    fn apply_state(index: Self::Repr, item: &Item<Box<[u8]>>, lookup: &mut Lookup) -> Result<(), snapshot::ApplyError<Lookup::Error>>;

    #[doc(hidden)]
    fn do_apply_state<T>(item: &Item<Box<[u8]>>, lookup: &mut Lookup) -> Result<(), snapshot::ApplyError<Lookup::Error>> 
    where 
        Lookup: item::lookup::OfTypeMut<T>,
        T: serde::de::DeserializeOwned,
    {
        let id = item::IdT::new(item.id().untyped());
        let version = item.version();
        let value = rmp_serde::from_slice::<T>(&*item.get())?;
        let item = Item::new(id, version, value);
        lookup.create(item)
            .map_err(snapshot::ApplyError::Lookup)?;
        Ok(())
    }
}
