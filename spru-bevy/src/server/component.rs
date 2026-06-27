use std::collections::VecDeque;

use bevy::prelude;
use derive_where::derive_where;

use crate::{common, server};

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
#[require(FromClient<Server>, ToClient<Server>)]
#[component(on_add, on_remove, on_despawn)]
pub struct Runner<Server: server::ServerSSS> {
    pub(crate) server: Server,
}

impl<Server: server::ServerSSS> Runner<Server> {
    pub(crate) fn new(server: Server) -> Self {
        Self { server }
    }

    pub fn save(&self) -> Result<spru::server::Save<Server>, spru::server::error::SaveError>
    where
        Server::PlayerInit: Clone,
        Server::Reaction: Clone,
    {
        self.server.save()
    }

    pub fn storage(
        &self,
    ) -> &spru::item::storage::Canonical<<Server::State as tagset::TagSet>::Repr, Server::State>
    {
        self.server.storage()
    }

    fn on_add(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        let game_id = **world.get::<common::component::GameId>(context.entity)
            .expect("Expected GameId");
        world.resource_mut::<server::resource::ServerMap>()
            .insert(game_id, context.entity);
    }

    fn on_remove(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        world.resource_mut::<server::resource::ServerMap>()
            .remove(context.entity);
    }

    fn on_despawn(mut world: bevy::ecs::world::DeferredWorld, context: bevy::ecs::lifecycle::HookContext) {
        world.resource_mut::<server::resource::ServerMap>()
            .remove(context.entity);
    }
}

#[derive_where(Debug; spru::common::signal::ToServer<Server::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct FromClient<Server: server::ServerSSS> {
    queues: Vec<(
        crate::reflect::spru::player::Id,
        VecDeque<spru::common::signal::ToServer<Server::Common>>,
    )>,
}

impl<Server: server::ServerSSS> FromClient<Server> {
    pub fn len(&self) -> usize {
        self.queues.len()
    }

    pub fn is_empty(&self) -> bool {
        for (_, queue) in &self.queues {
            if !queue.is_empty() {
                return false;
            }
        }
        true
    }

    pub fn enqueue(
        &mut self,
        client_id: spru::player::Id,
        signal: spru::common::signal::ToServer<Server::Common>,
    ) {
        get_queue_mut(client_id, &mut self.queues).push_back(signal);
    }

    pub(crate) fn dequeue_any(
        &mut self,
    ) -> Option<(
        spru::player::Id,
        spru::common::signal::ToServer<Server::Common>,
    )> {
        for (client_id, queue) in &mut self.queues {
            if let Some(signal) = queue.pop_front() {
                return Some((client_id.0, signal));
            }
        }
        None
    }
}

#[derive_where(Debug; spru::common::signal::ToClient<Server::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct ToClient<Server: server::ServerSSS> {
    queues: Vec<(
        crate::reflect::spru::player::Id,
        VecDeque<spru::common::signal::ToClient<Server::Common>>,
    )>,
}

impl<Server: server::ServerSSS> ToClient<Server> {
    pub fn len(&self) -> usize {
        self.queues.len()
    }

    pub fn is_empty(&self) -> bool {
        for (_, queue) in &self.queues {
            if !queue.is_empty() {
                return false;
            }
        }
        true
    }

    pub(crate) fn enqueue_outbound(
        &mut self,
        outbound: Vec<(spru::player::Id, spru::common::signal::ToClient<Server::Common>)>,
    ) {
        for (client_id, signal) in outbound {
            self.enqueue(client_id, signal);
        }
    }

    pub(crate) fn enqueue(
        &mut self,
        client_id: spru::player::Id,
        signal: spru::common::signal::ToClient<Server::Common>,
    ) {
        prelude::trace!("Signal to {client_id}");
        get_queue_mut(client_id, &mut self.queues).push_back(signal);
    }

    pub fn dequeue(
        &mut self,
        client_id: spru::player::Id,
    ) -> Option<spru::common::signal::ToClient<Server::Common>> {
        get_queue_mut(client_id, &mut self.queues).pop_front()
    }

    pub fn dequeue_any(
        &mut self,
    ) -> Option<(
        spru::player::Id,
        spru::common::signal::ToClient<Server::Common>,
    )> {
        for (client_id, queue) in &mut self.queues {
            if let Some(signal) = queue.pop_front() {
                return Some((client_id.0, signal));
            }
        }
        None
    }
}

fn get_queue_mut<T>(
    player_id: spru::player::Id,
    queues: &mut Vec<(crate::reflect::spru::player::Id, VecDeque<T>)>,
) -> &mut VecDeque<T> {
    let mut index = None;
    for (i, (queue_player_id, _queue)) in queues.iter_mut().enumerate() {
        if player_id == queue_player_id.0 {
            index = Some(i);
            break;
        }
    }

    let index = match index {
        Some(index) => index,
        None => {
            queues.push((crate::reflect::spru::player::Id(player_id), VecDeque::new()));
            queues.len() - 1
        }
    };

    &mut queues[index].1
}
