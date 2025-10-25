use std::{any, marker::PhantomData};

use bevy::prelude;
use spru::item;

use crate::{client::component, common};

#[doc(hidden)]
#[derive(Debug)]
pub struct BevyLookup<'l, State> {
    world: &'l mut prelude::World,
    entity_map: &'l mut super::component::EntityMap,
    game_id: spru::game::Id,
    client_id: spru::player::Id,
    _state: PhantomData<fn() -> State>,
}

impl<'l, State> BevyLookup<'l, State> {
    pub(crate) fn new(
        world: &'l mut prelude::World,
        entity_map: &'l mut super::component::EntityMap,
        game_id: spru::game::Id,
        client_id: spru::player::Id,
    ) -> Self {
        Self {
            world,
            entity_map,
            game_id,
            client_id,
            _state: PhantomData,
        }
    }
}

impl<'l, State: spru::State> spru::item::Lookup for BevyLookup<'l, State> {
    type State = State;

    fn lookup<T>(&self, id: item::IdT<T>) -> spru::item::lookup::Result<&spru::Item<T>>
    where
        T: spru::item::lookup::Lookupable<Self::State>,
    {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self
            .world
            .get::<component::Item<T>>(entity)
            .ok_or(super::BevyError::ComponentNotFound(
                id,
                entity,
                any::type_name::<T>(),
            ))?
            .item())
    }

    #[allow(refining_impl_trait)]
    fn lookup_mut<T>(
        &mut self,
        id: item::IdT<T>,
    ) -> spru::item::lookup::Result<bevy::prelude::Mut<'_, spru::Item<T>>>
    where
        T: spru::item::lookup::Lookupable<Self::State>,
    {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self
            .world
            .get_mut::<component::Item<T>>(entity)
            .ok_or(super::BevyError::ComponentNotFound(
                id,
                entity,
                any::type_name::<T>(),
            ))?
            .map_unchanged(|sc| sc.item_mut()))
    }

    fn create<T>(&mut self, value: spru::Item<T>) -> spru::item::lookup::Result<()>
    where
        T: spru::item::lookup::Lookupable<Self::State>,
    {
        let id = value.id().untyped();
        self.entity_map.insert_as(id, || {
            Ok(self
                .world
                .spawn((
                    prelude::Name::new(format!(
                        "[{:x}:{}.{id}] {}",
                        self.game_id.friendly_display(),
                        self.client_id,
                        any::type_name::<T>()
                    )),
                    component::Item::new(value),
                    common::component::GameId(self.game_id),
                    super::component::ClientId(self.client_id),
                ))
                .id())
        })?;
        Ok(())
    }

    fn destroy<T>(&mut self, id: item::IdT<T>) -> spru::item::lookup::Result<spru::Item<T>>
    where
        T: spru::item::lookup::Lookupable<Self::State>,
    {
        let id = id.untyped();
        self.entity_map
            .remove_as(id, |entity| {
                let mut entity_mut = self
                    .world
                    .get_entity_mut(entity)
                    .map_err(|_| super::BevyError::EntityNotFound(id, entity))?;
                match entity_mut.take::<component::Item<T>>() {
                    Some(item) => Ok(item.into_inner()),
                    None => Err(super::BevyError::ComponentNotFound(
                        id,
                        entity,
                        any::type_name::<T>(),
                    )),
                }
            })
            .map_err(Into::into)
    }
}
