use std::{any, collections::{hash_map, HashMap}};

use bevy::prelude;
use spru::item;

use crate::client::component;

#[derive(Debug)]
pub struct BevyLookup<'l> {
    world: &'l mut prelude::World,
    entity_map: &'l mut EntityMap,
    client_id: spru::player::Id,
}

impl<'l> BevyLookup<'l> {
    pub(crate) fn new(world: &'l mut prelude::World, entity_map: &'l mut EntityMap, client_id: spru::player::Id) -> Self {
        Self { 
            world,
            entity_map,
            client_id,
        }
    }
}

impl<'l, T: Send + Sync + 'static> spru::item::Lookup<T> for BevyLookup<'l> {
    type Mut<'lr> = bevy::prelude::Mut<'lr, spru::Item<T>>
    where Self: 'lr;  

    fn lookup(&self, id: item::IdT<T>) -> spru::item::lookup::Result<&spru::Item<T>> {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self.world.get::<component::Item<T>>(entity).ok_or(BevyError::ComponentNotFound(id, entity, any::type_name::<T>()))?.item())
    }

    fn lookup_mut(&mut self, id: item::IdT<T>) -> spru::item::lookup::Result<Self::Mut<'_>> {
        let id = id.untyped();
        let entity = self.entity_map.get(id)?;
        Ok(self.world.get_mut::<component::Item<T>>(entity).ok_or(BevyError::ComponentNotFound(id, entity, any::type_name::<T>()))?.map_unchanged(|sc| sc.item_mut()))
    }

    fn create(&mut self, value: spru::Item<T>) -> spru::item::lookup::Result<()> {
        self.entity_map.insert_as(value.id().untyped(), 
            || {
                Ok(
                    self.world.spawn((
                        component::Item::new(value),
                        component::ClientId(self.client_id),
                    )).id()
                )
            }
        )?;
        Ok(())
    }

    fn destroy(&mut self, id: item::IdT<T>) -> spru::item::lookup::Result<spru::Item<T>> {
        let id = id.untyped();
        self.entity_map.remove_as(id, |entity| {
            let mut entity_mut = self.world.get_entity_mut(entity)
                .map_err(|_| BevyError::EntityNotFound(id, entity))?;
            match entity_mut.take::<component::Item<T>>() {
                Some(item) => Ok(item.into_inner()),
                None => Err(BevyError::ComponentNotFound(id, entity, any::type_name::<T>()).into()),
            }
        }).map_err(Into::into)
    }
}


#[derive(Debug, Clone)]
#[derive(thiserror::Error)]
pub enum BevyError {
    #[error("Item {0} does not exist")]
    IdNotFound(item::Id),
    #[error("Item {0} should exist, but the bevy entity ({1}) has been removed")]
    EntityNotFound(item::Id, prelude::Entity),
    #[error("Item {0} should exist, but the bevy component ({1} {2}) has been removed")]
    ComponentNotFound(item::Id, prelude::Entity, &'static str),
    #[error("Item {0} already exists")]
    IdAlreadyExists(item::Id, prelude::Entity),
}

pub type BevyResult<T> = std::result::Result<T, BevyError>;

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct EntityMap {
    map: HashMap<item::Id, prelude::Entity>,
}

impl EntityMap {
    pub fn get(&self, id: item::Id) -> BevyResult<prelude::Entity> {
        self.map.get(&id).copied().ok_or(BevyError::IdNotFound(id))
    }

    fn insert_as(&mut self, id: item::Id, f: impl FnOnce() -> BevyResult<prelude::Entity>) ->  BevyResult<prelude::Entity> {
        match self.map.entry(id) {
            hash_map::Entry::Occupied(oe) => Err(BevyError::IdAlreadyExists(id, *oe.get()).into()),
            hash_map::Entry::Vacant(ve) => {
                let entity = f()?;
                ve.insert(entity);
                Ok(entity)
            }
        }
    }

    fn remove_as<T>(&mut self, id: item::Id, f: impl FnOnce(prelude::Entity) -> BevyResult<T>) -> BevyResult<T> {
        match self.map.entry(id) {
            hash_map::Entry::Occupied(oe) => {
                let value = f(*oe.get())?;
                oe.remove();
                Ok(value)
            },
            hash_map::Entry::Vacant(_) => Err(BevyError::IdNotFound(id)),
        }
    }
}

impl Default for EntityMap {
    fn default() -> Self {
        Self { 
            map: Default::default(), 
        }
    }
}