use crate::payload::variant;

#[doc(hidden)]
pub struct Marker<const N: variant::Id>;

#[doc(hidden)]
pub trait ValidMarker {
    const N: variant::Id;
}

impl<const N: variant::Id> ValidMarker for Marker<N> {
    const N: variant::Id = N;
}

#[doc(hidden)]
pub trait Slot<Marker> { 
    #[doc(hidden)]
    type Variant;
}
