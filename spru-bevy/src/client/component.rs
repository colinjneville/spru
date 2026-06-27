use std::{
    collections::{HashMap, VecDeque, hash_map},
    fmt, ops,
};

use bevy::prelude;
use derive_where::derive_where;
use spru::item;

use crate::{client, common};

/// Specifies the Client the entity belongs to. This allows multiple Clients to co-exist
/// inside the same World. Note that while this uses a [spru::player::Id] as the id,
/// this does not mean the attached game piece belongs to that player, only that it
/// is their 'view' of the game piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(prelude::Component, prelude::Reflect)]
#[component(immutable)]
pub struct ClientId(
    #[reflect(remote = crate::reflect::spru::player::Id)]
    spru::player::Id
);

impl ClientId {
    pub(crate) fn new(id: spru::player::Id) -> Self {
        Self(id)
    }
}

impl ops::Deref for ClientId {
    type Target = spru::player::Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[component(on_add, on_remove, on_despawn)]
#[require(FromServer<Client>, ToServer<Client>, EntityMap)]
pub struct Runner<Client: super::ClientSSS> {
    pub(crate) client: Option<Client>,
}

impl<Client: super::ClientSSS> Runner<Client> {
    pub(crate) fn new(client: Client) -> Self {
        Self { client: Some(client) }
    }

    fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("Client was not replaced after use")
    }

    fn client_mut(&mut self) -> &mut Client {
        self.client
            .as_mut()
            .expect("Client was not replaced after use")
    }

    fn on_add(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        let game_id = **world.get::<common::component::GameId>(context.entity)
            .expect("Expected GameId");
        let client_id = **world.get::<client::component::ClientId>(context.entity)
            .expect("Expected ClientId");
        
        world.resource_mut::<client::resource::ClientMap>()
            .insert(game_id, client_id, context.entity);
    }

    fn on_remove(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        world.resource_mut::<client::resource::ClientMap>()
            .remove(context.entity);
    }

    fn on_despawn(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        world.resource_mut::<client::resource::ClientMap>()
            .remove(context.entity);
    }

    pub fn pending_interactions(&self) -> impl Iterator<Item = spru::interaction::Pending> {
        self.client().pending_interactions()
    }

    pub(crate) fn storage_scope<Ret, F: FnOnce(&mut Client, &mut client::storage::BevyStorage<Client::State>) -> Ret>(entity: &mut prelude::EntityWorldMut, f: F) 
        -> prelude::Result<Ret>
    {
        let (mut runner, mut entity_map, game_id, client_id) = entity.get_components_mut::<(
            &mut Self, 
            &mut client::component::EntityMap,
            &common::component::GameId,
            &client::component::ClientId,
        )>()?;
        let mut client = runner.client.take().expect("Runner Client must always be restored");
        let mut client_entity_map = std::mem::take(&mut *entity_map);
        let game_id = **game_id;
        let client_id = **client_id;
        // TODO restore client & map even if the scope panics
        let ret = entity.world_scope(|world| {
            let mut storage = client::storage::BevyStorage::<Client::State>::new(world, &mut client_entity_map, game_id, client_id);
            f(&mut client, &mut storage)
        });

        let (mut runner, mut entity_map) = entity.get_components_mut::<(
            &mut Self, 
            &mut client::component::EntityMap,
        )>()?;
        runner.client = Some(client);
        *entity_map = client_entity_map;

        Ok(ret)
    }
}

#[derive_where(Debug; spru::common::signal::ToClient<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct FromServer<Client: super::ClientSSS> {
    queue: VecDeque<spru::common::signal::ToClient<Client::Common>>,
}

impl<Client: super::ClientSSS> FromServer<Client> {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn enqueue(&mut self, signal: spru::common::signal::ToClient<Client::Common>) {
        self.queue.push_back(signal);
    }

    pub fn dequeue(&mut self) -> Option<spru::common::signal::ToClient<Client::Common>> {
        self.queue.pop_front()
    }

    pub fn take(&mut self) -> impl IntoIterator<Item = spru::common::signal::ToClient<Client::Common>> + 'static {
        std::mem::take(&mut self.queue)
    }
}

#[derive_where(Debug; spru::common::signal::ToServer<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct ToServer<Client: super::ClientSSS> {
    queue: VecDeque<spru::common::signal::ToServer<Client::Common>>,
}

impl<Client: super::ClientSSS> ToServer<Client> {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn enqueue_outbound(&mut self, outbound: Vec<spru::common::signal::ToServer<Client::Common>>) {
        for signal in outbound {
            self.enqueue(signal);
        }
    }

    pub(crate) fn enqueue(&mut self, signal: spru::common::signal::ToServer<Client::Common>) {
        self.queue.push_back(signal);
    }

    pub fn dequeue(&mut self) -> Option<spru::common::signal::ToServer<Client::Common>> {
        self.queue.pop_front()
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct Item<T: Send + Sync + 'static>(spru::Item<T>);

impl<T: Send + Sync + 'static> Item<T> {
    pub(crate) fn new(item: spru::Item<T>) -> Self {
        Self(item)
    }

    pub(crate) fn item(&self) -> &spru::Item<T> {
        &self.0
    }

    pub(crate) fn item_mut(&mut self) -> &mut spru::Item<T> {
        &mut self.0
    }

    pub(crate) fn into_inner(self) -> spru::Item<T> {
        self.0
    }
}

impl<T: Send + Sync + 'static> ops::Deref for Item<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.item()
    }
}

#[derive(Debug, Default)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct EntityMap {
    map: HashMap<crate::reflect::spru::item::Id, prelude::Entity>,
}

impl EntityMap {
    pub fn get<ID: Into<item::Id>>(&self, id: ID) -> super::BevyResult<prelude::Entity> {
        let id = id.into();
        self.map
            .get(&crate::reflect::spru::item::Id(id))
            .copied()
            .ok_or(super::BevyError::IdNotFound(id))
    }

    #[doc(hidden)]
    pub fn take(&mut self) -> Self {
        Self {
            map: std::mem::take(&mut self.map),
        }
    }

    #[doc(hidden)]
    pub fn untake(&mut self, taken: Self) {
        assert!(self.map.is_empty());
        self.map = taken.map;
    }

    pub(crate) fn insert_as(
        &mut self,
        id: item::Id,
        f: impl FnOnce() -> super::BevyResult<prelude::Entity>,
    ) -> super::BevyResult<prelude::Entity> {
        match self.map.entry(crate::reflect::spru::item::Id(id)) {
            hash_map::Entry::Occupied(oe) => Err(super::BevyError::IdAlreadyExists(id, *oe.get())),
            hash_map::Entry::Vacant(ve) => {
                let entity = f()?;
                ve.insert(entity);
                Ok(entity)
            }
        }
    }

    pub(crate) fn remove_as<T>(
        &mut self,
        id: item::Id,
        f: impl FnOnce(prelude::Entity) -> super::BevyResult<T>,
    ) -> super::BevyResult<T> {
        match self.map.entry(crate::reflect::spru::item::Id(id)) {
            hash_map::Entry::Occupied(oe) => {
                let value = f(*oe.get())?;
                oe.remove();
                Ok(value)
            }
            hash_map::Entry::Vacant(_) => Err(super::BevyError::IdNotFound(id)),
        }
    }
}

impl<ID: Into<item::Id>> ops::Index<ID> for EntityMap {
    type Output = prelude::Entity;

    fn index(&self, index: ID) -> &Self::Output {
        &self.map[&crate::reflect::spru::item::Id(index.into())]
    }
}
