use std::any;
use tagset::tagset_meta;

use crate::{snapshot, state, item, Item};

pub type Index = u32;

#[telety::telety(crate::state, alias_traits = "always")]
#[tagset_meta]
// #[meta(bounds(for<VAR> Lookup: item::Lookup<VAR>))]
#[meta(bounds(for<VAR> Self: tagset::TagSetDiscriminant<VAR>))]
pub trait State: tagset::TagSet<Repr: TryFrom<Index> + Into<Index>> + Sized {
    #[doc(hidden)]
    #[meta(default {
        match_by_discriminant!(index, T => Self::do_apply_state::<Lookup, T>(item, lookup))
    })]
    fn apply_state<Lookup>(index: Self::Repr, item: &Item<Box<[u8]>>, lookup: &mut Lookup) 
        -> Result<(), snapshot::ApplyError>
    where 
        Lookup: item::Lookup<State = Self>,
    ;

    #[doc(hidden)]
    fn do_apply_state<Lookup, T>(item: &Item<Box<[u8]>>, lookup: &mut Lookup) 
        -> Result<(), snapshot::ApplyError> 
    where 
        // Self: tagset::TagSetDiscriminant<T, Repr: Into<state::Index>>,
        // T: any::Any + serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
        Lookup: item::Lookup<State = Self>,
        T: item::lookup::Lookupable<Self> + serde::de::DeserializeOwned,
    {
        let id = item::IdT::new(item.id().untyped());
        let version = item.version();
        let value = rmp_serde::from_slice::<T>(&*item.get())?;
        let item = Item::new(id, version, value);
        lookup.create(item)?;
        
        Ok(())
    }
}
