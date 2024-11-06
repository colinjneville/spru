pub mod lookup;
pub use lookup::{BevyLookup, BevyLookupMut};

pub use spru::item::*;

use std::ops;

#[derive(bevy::prelude::Component)]
pub struct Component<T: Send + Sync + 'static>(Option<spru::Item<T>>);

impl<T: Send + Sync + 'static> Component<T> {
    fn new(stateful: spru::Item<T>) -> Self {
        Self(Some(stateful))
    }

    fn take(&mut self) -> spru::Item<T> {
        // There doesn't seem to be a way to reclaim `Component`s from Bevy 
        // even with `World` access, so just before removing the `Component``,
        // extract the `Stateful`
        self.0.take().unwrap()
    }

    fn stateful(&self) -> &spru::Item<T> {
        self.0.as_ref().unwrap()
    }

    fn stateful_mut(&mut self) -> &mut spru::Item<T> {
        self.0.as_mut().unwrap()
    }
}

impl<T: Send + Sync + 'static> ops::Deref for Component<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.stateful()
    }
}
