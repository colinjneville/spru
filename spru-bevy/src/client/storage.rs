use std::{any, marker::PhantomData};

use bevy::prelude;
use spru::item;

use crate::{client, common};

#[derive(Debug)]
pub(crate) struct BevyReadOnlyStorage<'l, State> {
    world: &'l prelude::World,
    entity_map: &'l super::component::EntityMap,
    game_id: spru::game::Id,
    client_id: spru::player::Id,
    _state: PhantomData<fn() -> State>,
}

impl<'l, State> BevyReadOnlyStorage<'l, State> {
    pub(crate) fn new(
        world: &'l prelude::World,
        entity_map: &'l super::component::EntityMap,
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

impl<'l, State: spru::State> spru::item::ReadOnlyStorage for BevyReadOnlyStorage<'l, State> {
    type State = State;

    fn get<T>(&self, id: item::IdT<T>) -> spru::item::storage::Result<&spru::Item<T>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self
            .world
            .get::<client::component::Item<T>>(entity)
            .ok_or(super::BevyError::ComponentNotFound(
                id,
                entity,
                any::type_name::<T>(),
            ))?
            .item())
    }
}

impl<'l, State: spru::State> spru::item::Storage for BevyReadOnlyStorage<'l, State> {
    #[allow(refining_impl_trait)]
    fn get_mut<T>(
        &mut self,
        _id: item::IdT<T>,
    ) -> spru::item::storage::Result<bevy::prelude::Mut<'_, spru::Item<T>>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        panic!("BevyReadOnlyStorage cannot mutate state")
    }

    fn create<T>(&mut self, _value: spru::Item<T>) -> spru::item::storage::Result<()>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        panic!("BevyReadOnlyStorage cannot mutate state")
    }

    fn destroy<T>(&mut self, _id: item::IdT<T>) -> spru::item::storage::Result<spru::Item<T>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        panic!("BevyReadOnlyStorage cannot mutate state")
    }
}


#[derive(Debug)]
pub struct BevyStorage<'l, State> {
    world: &'l mut prelude::World,
    entity_map: &'l mut super::component::EntityMap,
    game_id: spru::game::Id,
    client_id: spru::player::Id,
    _state: PhantomData<fn() -> State>,
}

impl<'l, State> BevyStorage<'l, State> {
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

impl<'l, State: spru::State> spru::item::ReadOnlyStorage for BevyStorage<'l, State> {
    type State = State;

    fn get<T>(&self, id: item::IdT<T>) -> spru::item::storage::Result<&spru::Item<T>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self
            .world
            .get::<client::component::Item<T>>(entity)
            .ok_or(super::BevyError::ComponentNotFound(
                id,
                entity,
                any::type_name::<T>(),
            ))?
            .item())
    }
}

impl<'l, State: spru::State> spru::item::Storage for BevyStorage<'l, State> {
    #[allow(refining_impl_trait)]
    fn get_mut<T>(
        &mut self,
        id: item::IdT<T>,
    ) -> spru::item::storage::Result<bevy::prelude::Mut<'_, spru::Item<T>>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self
            .world
            .get_mut::<client::component::Item<T>>(entity)
            .ok_or(super::BevyError::ComponentNotFound(
                id,
                entity,
                any::type_name::<T>(),
            ))?
            .map_unchanged(|sc| sc.item_mut()))
    }

    fn create<T>(&mut self, value: spru::Item<T>) -> spru::item::storage::Result<()>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        let id = value.id().untyped();
        self.entity_map.insert_as(id, || {
            Ok(self
                .world
                .spawn((
                    prelude::Name::new(format!(
                        "[{}:{}.{id}] {}",
                        self.game_id.friendly_display(),
                        self.client_id,
                        any::type_name::<T>()
                    )),
                    client::component::Item::new(value),
                    common::component::GameId::new(self.game_id),
                    super::component::ClientId::new(self.client_id),
                ))
                .id())
        })?;
        Ok(())
    }

    fn destroy<T>(&mut self, id: item::IdT<T>) -> spru::item::storage::Result<spru::Item<T>>
    where
        T: spru::item::storage::Storable<Self::State>,
    {
        let id = id.untyped();
        self.entity_map
            .remove_as(id, |entity| {
                let mut entity_mut = self
                    .world
                    .get_entity_mut(entity)
                    .map_err(|_| super::BevyError::EntityNotFound(id, entity))?;
                match entity_mut.take::<client::component::Item<T>>() {
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
