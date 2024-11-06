pub mod adapter;
pub use adapter::Adapter;
pub mod catalog;
pub use catalog::Catalog;

use crate::item;

pub struct Output<Undo, Out> {
    pub undo: Option<Undo>,
    pub out: Out,
}

impl<Undo, Out> From<(Undo, Out)> for Output<Undo, Out> {
    fn from((undo, out): (Undo, Out)) -> Self {
        Self {
            undo: Some(undo),
            out,
        }
    }
}

impl<Undo> From<(Option<Undo>, ())> for Output<Undo, ()> {
    fn from((undo, out): (Option<Undo>, ())) -> Self {
        Self {
            undo,
            out,
        }
    }
}

impl<Undo> From<Undo> for Output<Undo, ()> {
    fn from(undo: Undo) -> Self {
        Self {
            undo: Some(undo),
            out: (),
        }
    }
}

impl<Undo> From<Option<Undo>> for Output<Undo, ()> {
    fn from(undo: Option<Undo>) -> Self {
        Self {
            undo,
            out: (),
        }
    }
}

pub trait Base: Clone + crate::Serial {
    type Error;
    type Undo;
}

// TODO rename
pub trait Base2: Base {
    type Adapter: Adapter;
}

pub type In<'l, A, Lookup> = <<A as Base2>::Adapter as Adapter>::In<'l, <A as Action>::T, Lookup>;
pub type Out<A, Lookup> = <<A as Base2>::Adapter as Adapter>::Out<<A as Action>::T, Lookup>;

pub trait Action: Base2 {
    type T;
    // type Adapter: Adapter<Lookup>;

	// type Error;
	// type Undo;
	
	fn apply<'l, Lookup>(&self, input: In<'l, Self, Lookup>) -> Result<impl Into<Output<Self::Undo, Out<Self, Lookup>>>, Self::Error>
    where Lookup: item::lookup::OfTypeMut<Self::T> + 'l;
}

