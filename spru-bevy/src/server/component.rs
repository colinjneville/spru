use std::collections::VecDeque;

use bevy::prelude;
use derive_where::derive_where;

#[derive(Debug)]
#[derive(prelude::Component)]
#[require(FromClient<Server>, ToClient<Server>)]
pub struct Runner<Server: super::ServerSSS> {
    pub(crate) server: Server,
}

impl<Server: super::ServerSSS> Runner<Server> {
    pub(crate) fn new<'l, GameInit>(game_init: GameInit, player_init: Server::PlayerInit, reaction: Server::Reaction)
         -> spru::TempResult<Self> 
    where
        GameInit: spru::game::Init<State = Server::State, Action = Server::Action, Root = Server::Root>,
    {
        let server = Server::new(game_init, player_init, reaction)?;
        Ok(Self {
            server,
        })
    }

    pub(crate) fn load_from_save(save: spru::Save<Server::State, Server::Root, Server::PlayerInit, Server::Reaction>) 
        -> Result<Server, spru::server::LoadError>
    {
        Server::load_from_save(save)
    }

    pub(crate) fn create_save(&self) 
        -> Result<spru::Save<Server::State, Server::Root, Server::PlayerInit, Server::Reaction>, spru::snapshot::CreateError> 
    where
        Server::PlayerInit: Clone,
        Server::Reaction: Clone,
    {
        self.server.create_save()
    }

    pub(crate) fn add_player(&mut self, init_input: <Server::PlayerInit as spru::player::Init>::In) -> spru::server::add_player::Result<()> {
        let spru::server::Output {
            outbound,
            events,
            ret: spru::server::add_player::Ret {
                client_init,
                player_id,
            },
        } = self.server.add_player(spru::server::add_player::Arg {
            init_input,
        })?;
        
        todo!()
    }
}

#[derive(Debug)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct FromClient<Server: super::ServerSSS> {
    queues: Vec<(spru::player::Id, VecDeque<spru::server::signal::Arg<Server::Interaction>>)>,
}

impl<Server: super::ServerSSS> FromClient<Server> {
    pub fn enqueue(&mut self, client_id: spru::player::Id, signal: spru::server::signal::Arg<Server::Interaction>) {
        get_queue_mut(client_id, &mut self.queues)
            .push_back(signal);
    }

    pub(crate) fn dequeue_any(&mut self) -> Option<(spru::player::Id, spru::server::signal::Arg<Server::Interaction>)> {
        for (client_id, queue) in &mut self.queues {
            if let Some(signal) = queue.pop_front() {
                return Some((*client_id, signal));
            }
        }
        None
    }
}

#[derive_where(Debug; spru::client::signal::Arg<Server::Action, <Server::Reaction as spru::Reaction>::GameOutcome>)]
#[derive_where(Default)]
#[derive(prelude::Component)]
pub struct ToClient<Server: super::ServerSSS> {
    queues: Vec<(spru::player::Id, VecDeque<spru::client::signal::Arg<Server::Action, <Server::Reaction as spru::Reaction>::GameOutcome>>)>,
}

impl<Server: super::ServerSSS> ToClient<Server> {
    pub(crate) fn enqueue(&mut self, client_id: spru::player::Id, signal: spru::client::signal::Arg<Server::Action, <Server::Reaction as spru::Reaction>::GameOutcome>) {
        get_queue_mut(client_id, &mut self.queues)
            .push_back(signal);
    }

    pub fn dequeue(&mut self, client_id: spru::player::Id) -> Option<spru::client::signal::Arg<Server::Action, <Server::Reaction as spru::Reaction>::GameOutcome>> {
        get_queue_mut(client_id, &mut self.queues)
            .pop_front()
    }

    pub fn dequeue_any(&mut self) -> Option<(spru::player::Id, spru::client::signal::Arg<Server::Action, <Server::Reaction as spru::Reaction>::GameOutcome>)> {
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

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct PendingClients<Server: super::ServerSSS> {
    queue: VecDeque<spru::server::add_player::Ret<Server::State, Server::Action, Server::Root>>,
}

impl<Server: super::ServerSSS> PendingClients<Server> {
    pub(crate) fn enqueue(&mut self, ret: spru::server::add_player::Ret<Server::State, Server::Action, Server::Root>) {
        self.queue.push_back(ret);
    }

    pub fn dequeue(&mut self) -> Option<spru::server::add_player::Ret<Server::State, Server::Action, Server::Root>> {
        self.queue.pop_front()
    }
}