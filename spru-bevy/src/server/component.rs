use std::collections::VecDeque;

use bevy::prelude;
use derive_where::derive_where;

#[derive(Debug)]
#[derive(prelude::Component)]
#[require(FromClient<Server>, ToClient<Server>, FromUser<Server>, PendingClients<Server>)]
pub struct Runner<Server: super::ServerSSS> {
    pub(crate) server: Server,
}

impl<Server: super::ServerSSS> Runner<Server> {
    pub(crate) fn new(server: Server) -> Self {
        Self {
            server,
        }
    }

    pub fn save(&self) 
        -> spru::server::save::Result<Server> 
    where
        Server::PlayerInit: Clone,
        Server::Reaction: Clone,
    {
        self.server.save()
    }
}

#[derive_where(Debug; spru::server::signal::Signal<Server::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct FromClient<Server: super::ServerSSS> {
    queues: Vec<(spru::player::Id, VecDeque<spru::server::signal::Signal<Server::Common>>)>,
}

impl<Server: super::ServerSSS> FromClient<Server> {
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

    pub fn enqueue(&mut self, client_id: spru::player::Id, signal: spru::server::signal::Signal<Server::Common>) {
        get_queue_mut(client_id, &mut self.queues)
            .push_back(signal);
    }

    pub(crate) fn dequeue_any(&mut self) -> Option<(spru::player::Id, spru::server::signal::Signal<Server::Common>)> {
        for (client_id, queue) in &mut self.queues {
            if let Some(signal) = queue.pop_front() {
                return Some((*client_id, signal));
            }
        }
        None
    }
}

#[derive_where(Debug; spru::client::signal::Signal<Server::Common>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct ToClient<Server: super::ServerSSS> {
    queues: Vec<(spru::player::Id, VecDeque<spru::client::signal::Signal<Server::Common>>)>,
}

impl<Server: super::ServerSSS> ToClient<Server> {
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

    pub(crate) fn enqueue(&mut self, client_id: spru::player::Id, signal: spru::client::signal::Signal<Server::Common>) {
        get_queue_mut(client_id, &mut self.queues)
            .push_back(signal);
    }

    pub fn dequeue(&mut self, client_id: spru::player::Id) -> Option<spru::client::signal::Signal<Server::Common>> {
        get_queue_mut(client_id, &mut self.queues)
            .pop_front()
    }

    pub fn dequeue_any(&mut self) -> Option<(spru::player::Id, spru::client::signal::Signal<Server::Common>)> {
        for (client_id, queue) in &mut self.queues {
            if let Some(signal) = queue.pop_front() {
                return Some((*client_id, signal));
            }
        }
        None
    }
}

fn get_queue_mut<T>(player_id: spru::player::Id, queues: &mut Vec<(spru::player::Id, VecDeque<T>)>) -> &mut VecDeque<T> {
    let mut index = None;
    for (i, (queue_player_id, _queue)) in queues.into_iter().enumerate() {
        if player_id == *queue_player_id {
            index = Some(i);
            break;
        }
    }

    let index = match index {
        Some(index) => index,
        None => {
            queues.push((player_id, VecDeque::new()));
            queues.len() - 1
        }
    };
    
    &mut queues[index].1
}

#[derive_where(Default; )]
#[derive_where(Debug; spru::server::add_player::Ret<Server>)]
#[derive(prelude::Component)]
pub struct PendingClients<Server: super::ServerSSS> {
    queue: VecDeque<spru::server::add_player::Ret<Server>>,
}

impl<Server: super::ServerSSS> PendingClients<Server> {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn enqueue(&mut self, ret: spru::server::add_player::Ret<Server>) {
        self.queue.push_back(ret);
    }

    pub fn dequeue(&mut self) -> Option<spru::server::add_player::Ret<Server>> {
        self.queue.pop_front()
    }
}


#[derive_where(Debug; UserInput<Server>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct FromUser<Server: super::ServerSSS> {
    queue: VecDeque<UserInput<Server>>,
}

impl<Server: super::ServerSSS> FromUser<Server> {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn add_player(&mut self, user_init_in: <Server::PlayerInit as spru::player::Init>::In) {
        self.queue.push_back(UserInput::AddPlayer(user_init_in));
    }

    pub fn manual_trigger(&mut self, trigger: <Server::Reaction as spru::Reaction>::Trigger) {
        self.queue.push_back(UserInput::ManualTrigger(trigger));
    }

    pub(crate) fn dequeue(&mut self) -> Option<UserInput<Server>> {
        self.queue.pop_front()
    }
}

#[derive_where(Debug; 
    <Server::PlayerInit as spru::player::Init>::In, 
    <Server::Reaction as spru::Reaction>::Trigger,
)]
pub(crate) enum UserInput<Server: super::ServerSSS> {
    AddPlayer(<Server::PlayerInit as spru::player::Init>::In),
    ManualTrigger(<Server::Reaction as spru::Reaction>::Trigger),
}