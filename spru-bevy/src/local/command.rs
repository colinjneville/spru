use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::{client, server};

#[derive_where(Debug; <Server::PlayerInit as spru::player::Init>::In)]
pub struct AddLocalPlayer<Server: server::ServerSSS, Client: client::ClientSSS> {
    input: <Server::PlayerInit as spru::player::Init>::In,
    _p: PhantomData<Client>,
}

impl<Server: server::ServerSSS, Client: client::ClientSSS> AddLocalPlayer<Server, Client> {
    pub fn new(input: <Server::PlayerInit as spru::player::Init>::In) -> Self {
        Self {
            input,
            _p: PhantomData,
        }
    }
}

impl<Server: server::ServerSSS<Common = Client::Common>, Client: client::ClientSSS> prelude::EntityCommand for AddLocalPlayer<Server, Client> {
    type Out = prelude::Result;
    
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            input,
            _p,
        } = self;

        let result = entity
            .get_components_mut::<&mut server::component::Runner<Server>>()?
            .server.add_player(input);

        let result = match result {
            Ok(output) => {
                let spru::server::Output {
                    outbound,
                    events,
                    ret,
                } = output;
                let seed = ret;
                
                server::trigger_events(entity.id(), &mut entity, events);

                entity.get_components_mut::<&mut server::component::ToClient<Server>>()?
                    .enqueue_outbound(outbound);

                let player_id = seed.local_player_id();
                entity
                    .world_scope(|world| {
                        crate::client::command::Init::<Client> { seed }.apply(world.spawn_empty())
                    });

                Ok(player_id)
            }
            Err(err) => Err(err),
        };

        entity.trigger(|entity| server::event::AddPlayer::<Server> {
            entity,
            result,
            _server: PhantomData,
        });

        Ok(())
    }
}

