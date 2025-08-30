pub use spru::item::lookup::*;

use std::{any, collections::{hash_map, HashMap}, ops};

use bevy::prelude::{Entity, Resource, World};

use crate::{item, Item};

pub struct BevyLookup<'l> {
    world: &'l World,
}

impl<'l> BevyLookup<'l> {
    pub fn new(world: &'l World) -> Self {
        Self { world }
    }

    pub(crate) fn entity_map(&self) -> &EntityMap {
        self.world.resource()
    }
}

impl<'l> spru::item::Lookup for BevyLookup<'l> {
    type Error = BevyError;
}

impl<'l, T: Send + Sync + 'static> spru::item::lookup::Lookup<T> for BevyLookup<'l> {
    fn lookup(&self, id: item::IdT<T>) -> Result<&spru::Item<T>, Self::Error> {
        let id = id.untyped();
        let entity = self.world.resource::<EntityMap>().get(id)?;
        Ok(self.world.get::<item::Component<T>>(entity).ok_or(BevyError::ComponentNotFound(id, entity, any::TypeId::of::<T>()))?.item())
    }
}

pub struct BevyLookupMut<'l> {
    world: &'l mut World,
}

impl<'l> BevyLookupMut<'l> {
    pub fn new(world: &'l mut World) -> Self {
        Self { world }
    }

    pub(crate) fn entity_map(&self) -> &EntityMap {
        self.world.resource()
    }

    pub(crate) fn entity_map_mut(&mut self) -> impl ops::DerefMut<Target=EntityMap> + '_ {
        self.world.resource_mut()
    }
}


impl<'l> spru::item::Lookup for BevyLookupMut<'l> {
    type Error = BevyError;
}

impl<'l, T: Send + Sync + 'static> spru::item::lookup::Lookup<T> for BevyLookupMut<'l> {
    type Mut<'lr> = bevy::prelude::Mut<'lr, Item<T>>
    where Self: 'lr;  

    fn lookup(&self, id: item::IdT<T>) -> Result<&Item<T>, Self::Error> {
        println!("Looking up {:?}", id);
        let id = id.untyped();
        let entity = self.world.resource::<EntityMap>().get(id)?;
        Ok(self.world.get::<item::Component<T>>(entity).ok_or(BevyError::ComponentNotFound(id, entity, any::TypeId::of::<T>()))?.item())
    }

    fn lookup_mut(&mut self, id: item::IdT<T>) -> Result<Self::Mut<'_>, Self::Error> {
        println!("Looking up mut {:?}", id);
        let id = id.untyped();
        let entity = self.world.resource::<EntityMap>().get(id)?;
        Ok(self.world.get_mut::<item::Component<T>>(entity).ok_or(BevyError::ComponentNotFound(id, entity, any::TypeId::of::<T>()))?.map_unchanged(|sc| sc.item_mut()))
    }

    fn put(&mut self, value: Item<T>) -> Result<(), Self::Error> {
        self.world.resource_scope::<EntityMap, _>(|world, mut entity_map| {
            println!("Creating {:?} {}", value.id(), value.version());
            entity_map.insert_as(value.id().untyped(), || Ok(world.spawn(item::Component::new(value)).id())).map(|_| ())
        })
    }

    fn take(&mut self, id: item::IdT<T>) -> Result<Item<T>, Self::Error> {
        let id = id.untyped();
        self.world.resource_scope::<EntityMap, _>(|world, mut entity_map| {
            entity_map.remove_as(id, |entity| {
                let mut entity_mut = world.get_entity_mut(entity)
                    .map_err(|_| BevyError::EntityNotFound(id, entity))?;
                match entity_mut.take::<item::Component<T>>() {
                    Some(crate::item::Component(item)) => Ok(item),
                    None => Err(BevyError::ComponentNotFound(id, entity, any::TypeId::of::<T>())),
                }
            })
        })
    }

    fn create(&mut self, value: Item<T>) -> Result<(), Self::Error> {
        // TODO change tracking
        self.put(value)
    }

    fn destroy(&mut self, id: item::IdT<T>) -> Result<Item<T>, Self::Error> {
        // TODO change tracking
        self.take(id)
    }
}


#[derive(Debug, Clone)]
pub enum BevyError {
    IdNotFound(item::Id),
    EntityNotFound(item::Id, Entity),
    ComponentNotFound(item::Id, Entity, any::TypeId),
    IdAlreadyExists(item::Id, Entity),
}

#[derive(Debug)]
#[derive(Resource)]
pub struct EntityMap {
    map: HashMap<item::Id, Entity>,
}

impl EntityMap {
    pub fn get(&self, id: item::Id) -> Result<Entity, BevyError> {
        self.map.get(&id).copied().ok_or(BevyError::IdNotFound(id))
    }

    // fn insert(&mut self, entity: Entity) -> Id {

    //     let id = self.next_id;
    //     self.map.insert(id, entity);
    //     self.next_id = self.next_id.next();
    //     id
    // }

    fn insert_as(&mut self, id: item::Id, f: impl FnOnce() -> Result<Entity, BevyError>) ->  Result<Entity, BevyError> {
        match self.map.entry(id) {
            hash_map::Entry::Occupied(oe) => Err(BevyError::IdAlreadyExists(id, *oe.get())),
            hash_map::Entry::Vacant(ve) => {
                let entity = f()?;
                ve.insert(entity);
                Ok(entity)
            }
        }
    }

    fn remove_as<T>(&mut self, id: item::Id, f: impl FnOnce(Entity) -> Result<T, BevyError>) -> Result<T, BevyError> {
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