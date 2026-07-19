use std::{marker::PhantomData, sync::Arc};

use bevy::prelude;
use derive_where::derive_where;

use crate::{client, local, server};

#[derive_where(Debug; <Server::PlayerInit as spru::player::Init>::In)]
pub struct AddLocalPlayer<Server: server::ServerSSS, Client: client::ClientSSS> {
    input: <Server::PlayerInit as spru::player::Init>::In,
    client_entity: Option<prelude::Entity>,
    _p: PhantomData<Client>,
}

impl<Server: server::ServerSSS, Client: client::ClientSSS> AddLocalPlayer<Server, Client> {
    pub fn new(input: <Server::PlayerInit as spru::player::Init>::In) -> Self {
        Self {
            input,
            client_entity: None,
            _p: PhantomData,
        }
    }

    pub fn new_for_entity(input: <Server::PlayerInit as spru::player::Init>::In, client_entity: prelude::Entity) -> Self {
        Self {
            input,
            client_entity: Some(client_entity),
            _p: PhantomData,
        }
    }
}

impl<Server: server::ServerSSS<Common = Client::Common>, Client: client::ClientSSS> prelude::EntityCommand for AddLocalPlayer<Server, Client> 
where 
    Server::PlayerInit: spru::player::Init<In: Clone>,
{
    type Out = prelude::Result;
    
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            input,
            client_entity,
            _p,
        } = self;

        let result = entity
            .get_components_mut::<&mut server::component::Runner<Server>>()?
            .server.add_player(input.clone());

        match result {
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
                let client_entity = entity
                    .world_scope(|world| {
                        let entity_commands = if let Some(client_entity) = client_entity {
                            world.entity_mut(client_entity)
                        } else {
                            world.spawn_empty()
                        };
                        let client_entity = entity_commands.id();
                        crate::client::command::Init::<Client> { seed }.apply(entity_commands);
                        client_entity
                    });

                entity
                    .trigger(|entity| server::event::PlayerAdded {
                        entity,
                        player_id,
                    })
                    .trigger(|server_entity| local::event::LocalPlayerAdded {
                        server_entity,
                        client_entity,
                        player_id,
                    });
            }
            Err(error) => {
                let error = Arc::new(error);

                entity
                    .trigger(|entity| server::event::PlayerAddError {
                        entity,
                        error: error.clone(),
                    })
                    .trigger(|server_entity| local::event::LocalPlayerAddError {
                        server_entity,
                        error,
                    })
                    ;
            },
        };

        Ok(())
    }
}

