pub mod lookup;
pub use lookup::{BevyLookup, BevyLookupMut};

pub use spru::item::*;

use std::ops;

#[derive(bevy::prelude::Component)]
pub struct Component<T: Send + Sync + 'static>(spru::Item<T>);

impl<T: Send + Sync + 'static> Component<T> {
    fn new(item: spru::Item<T>) -> Self {
        Self(item)
    }

    fn item(&self) -> &spru::Item<T> {
        &self.0
    }

    fn item_mut(&mut self) -> &mut spru::Item<T> {
        &mut self.0
    }
}

impl<T: Send + Sync + 'static> ops::Deref for Component<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.item()
    }
}
