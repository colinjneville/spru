use std::{collections::VecDeque, ops};

use bevy::prelude;
use derive_where::derive_where;

/// Specifies the Client the entity belongs to. This allows multiple Clients to co-exist 
/// inside the same World. Note that while this uses a [spru::player::Id] as the id,
/// this does not mean the attached game piece belongs to that player, only that it
/// is their 'view' of thr game piece.
#[derive(Debug)]
#[derive(prelude::Component)]
pub struct ClientId(pub spru::player::Id);

#[derive_where(Debug; RunnerInner<Client>)]
#[derive(prelude::Component)]
#[require(FromServer<Client>, ToServer<Client>)]
pub struct Runner<Client: super::ClientSSS> {
    inner: Option<RunnerInner<Client>>,
}

impl<Client: super::ClientSSS> Runner<Client> {
    pub(crate) fn new(world: &mut bevy::prelude::World, init: spru::client::init::Arg<Client::Common>) 
        -> spru::client::init::Result<Self> 
    {
        let mut entity_map = super::lookup::EntityMap::default();
        let mut lookup = super::lookup::BevyLookup::new(world, &mut entity_map, init.local_player_id());
        let client = Client::init(&mut lookup, init)?;
            
        Ok(Self {
            inner: Some(RunnerInner {
                client,
                entity_map,
            })
        })
    }

    pub(crate) fn inner(&self) -> &RunnerInner<Client> {
        self.inner.as_ref()
            .expect("Runner must be restored")
    }

    pub(crate) fn take(&mut self) -> RunnerInner<Client> {
        self.inner.take()
            .expect("Runner must be restored")
    }

    pub(crate) fn restore(&mut self, inner: RunnerInner<Client>) {
        self.inner = Some(inner);
    }
}

#[derive(Debug)]
pub(crate) struct RunnerInner<Client: super::ClientSSS> {
    pub(crate) client: Client,
    pub(crate) entity_map: super::lookup::EntityMap,
}

#[derive_where(Debug; spru::client::signal::Arg<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct FromServer<Client: super::ClientSSS> {
    queue: VecDeque<spru::client::signal::Arg<Client::Common>>,
}

impl<Client: super::ClientSSS> FromServer<Client> {
    pub(crate) fn enqueue(&mut self, signal: spru::client::signal::Arg<Client::Common>) {
        self.queue
            .push_back(signal);
    }

    pub fn dequeue(&mut self) -> Option<spru::client::signal::Arg<Client::Common>> {
        self.queue
            .pop_front()
    }
}

#[derive_where(Debug; spru::server::signal::Arg<Client::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct ToServer<Client: super::ClientSSS> {
    queue: VecDeque<spru::server::signal::Arg<Client::Common>>,
}

impl<Client: super::ClientSSS> ToServer<Client> {
    pub fn enqueue(&mut self, signal: spru::server::signal::Arg<Client::Common>) {
        self.queue
            .push_back(signal);
    }

    pub(crate) fn dequeue(&mut self) -> Option<spru::server::signal::Arg<Client::Common>> {
        self.queue
            .pop_front()
    }
}

#[derive(Debug)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct ApplyInteraction<Client: super::ClientSSS> {
    queue: VecDeque<Client::Interaction>,
}

#[derive(Debug)]
#[derive(prelude::Component)]
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
        &*self.item()
    }
}
