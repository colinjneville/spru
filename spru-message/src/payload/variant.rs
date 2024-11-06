use crate::payload;

pub type Id = u8;

pub trait Variant<V>: payload::Slot<Self::Marker, Variant = V> {
    #[doc(hidden)]
    type Marker: payload::slot::ValidMarker;

    #[doc(hidden)]
    fn variant_id() -> Id {
        <Self::Marker as payload::slot::ValidMarker>::N
    }
}