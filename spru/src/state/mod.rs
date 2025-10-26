use tagset::tagset_meta;

use crate::{common, item};

/// A set of all the possible types an [crate::Item] could contain for a game.
/// Implement [State] using [tagset::tagset]:
/// ```rust,ignore
/// # use tagset::tagset;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct Counter(i32);
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct Whiteboard(String);
///
/// #[tagset(impl tagset::proxy::serde::Serialize)]
/// #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
/// #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
/// #[tagset(impl spru::State)]
/// #[tagset(Counter)]
/// #[tagset(Whiteboard)]
/// struct MyState;
///
/// # fn main() { }
/// ```
/// Here `Counter` and `Whiteboard` are included in `MyState` and can be used as items tracked by spru.
/// For generic types, you must specify each monomorphized type you want to use, e.g. `Counter<u8> and `Counter<i32>`.
#[telety::telety(crate::state, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(for<VAR> Self: tagset::TagSetDiscriminant<VAR>))]
pub trait State: tagset::TagSet<Repr: Clone + Eq + std::hash::Hash> + Sized {
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
