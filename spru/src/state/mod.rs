use tagset::tagset_meta;

use crate::{common, item};

pub type Index = u32;

#[telety::telety(crate::state, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(for<VAR> Self: tagset::TagSetDiscriminant<VAR>))]
pub trait State: tagset::TagSet<Repr: TryFrom<Index> + Into<Index>> + Sized {
    #[doc(hidden)]
    #[meta(default {
        match_by_discriminant!(index, T => item.cast::<Lookup, T>(lookup))
    })]
    fn apply_state<Lookup>(
        index: Self::Repr,
        item: &item::Erased,
        lookup: &mut Lookup,
    ) -> Result<(), common::error::Load>
    where
        Lookup: item::Lookup<State = Self>;
}
