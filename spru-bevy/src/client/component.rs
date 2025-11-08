use std::{
    collections::{HashMap, VecDeque, hash_map},
    fmt, ops,
};

use bevy::prelude;
use derive_where::derive_where;
use spru::item;

/// Specifies the Client the entity belongs to. This allows multiple Clients to co-exist
/// inside the same World. Note that while this uses a [spru::player::Id] as the id,
/// this does not mean the attached game piece belongs to that player, only that it
/// is their 'view' of the game piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, prelude::Component)]
#[component(storage = "SparseSet")]
#[component(immutable)]
pub struct ClientId(spru::player::Id);

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

#[derive(Debug, prelude::Component)]
#[component(storage = "SparseSet")]
#[require(FromServer<Client>, ToServer<Client>, FromUser<Client>, EntityMap)]
pub struct Runner<Client: super::ClientSSS> {
    pub(crate) client: Client,
}

impl<Client: super::ClientSSS> Runner<Client> {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }
}

#[derive_where(Debug; spru::common::signal::ToClient<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
#[component(storage = "SparseSet")]
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
}

#[derive_where(Debug; spru::common::signal::ToServer<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
#[component(storage = "SparseSet")]
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

    pub fn enqueue(&mut self, signal: spru::common::signal::ToServer<Client::Common>) {
        self.queue.push_back(signal);
    }

    pub(crate) fn dequeue(&mut self) -> Option<spru::common::signal::ToServer<Client::Common>> {
        self.queue.pop_front()
    }
}

#[derive_where(Debug; UserInput<Client>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
#[component(storage = "SparseSet")]
pub struct FromUser<Client: super::ClientSSS> {
    queue: VecDeque<UserInput<Client>>,
}

impl<Client: super::ClientSSS> FromUser<Client> {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn stage_interaction(&mut self, interaction: Client::Interaction) {
        self.queue
            .push_back(UserInput::StageInteraction(interaction));
    }

    pub fn apply_interaction(&mut self, interaction_id: spru::interaction::Pending) {
        self.queue
            .push_back(UserInput::ApplyInteraction(Some(interaction_id)));
    }

    pub fn apply_all_interactions(&mut self) {
        self.queue.push_back(UserInput::ApplyInteraction(None));
    }

    pub fn revert_interaction(&mut self, interaction_id: spru::interaction::Pending) {
        self.queue
            .push_back(UserInput::RevertInteraction(Some(interaction_id)));
    }

    pub fn revert_all_interactions(&mut self) {
        self.queue.push_back(UserInput::RevertInteraction(None));
    }

    pub(crate) fn dequeue(&mut self) -> Option<UserInput<Client>> {
        self.queue.pop_front()
    }
}

#[allow(
    clippy::enum_variant_names,
    reason = "Future variants will not involve interactions"
)]
#[derive_where(Debug; Client::Interaction)]
pub(crate) enum UserInput<Client: super::ClientSSS> {
    StageInteraction(Client::Interaction),
    ApplyInteraction(Option<spru::interaction::Pending>),
    RevertInteraction(Option<spru::interaction::Pending>),
}

#[derive(Debug, prelude::Component)]
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

#[derive(Debug, Default, prelude::Component)]
#[component(storage = "SparseSet")]
pub struct EntityMap {
    map: HashMap<item::Id, prelude::Entity>,
}

impl EntityMap {
    pub fn get<ID: Into<item::Id>>(&self, id: ID) -> super::BevyResult<prelude::Entity> {
        let id = id.into();
        self.map
            .get(&id)
            .copied()
            .ok_or(super::BevyError::IdNotFound(id))
    }

    pub(crate) fn insert_as(
        &mut self,
        id: item::Id,
        f: impl FnOnce() -> super::BevyResult<prelude::Entity>,
    ) -> super::BevyResult<prelude::Entity> {
        match self.map.entry(id) {
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
        match self.map.entry(id) {
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
        &self.map[&index.into()]
    }
}
