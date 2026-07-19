use std::{marker::PhantomData, sync::Arc};

use bevy::prelude;
use derive_where::derive_where;
use tracing::instrument;

use crate::{common, server};

#[derive_where(Debug; GameInit, Server::PlayerInit, Server::Reaction)]
pub struct Init<Server: super::ServerSSS, GameInit> {
    pub game_init: GameInit,
    pub player_init: Server::PlayerInit,
    pub reaction: Server::Reaction,
}

impl<Server, GameInit> prelude::EntityCommand for Init<Server, GameInit>
where
    Server: super::ServerSSS,
    GameInit: spru::game::Init<Action = Server::Action, Root = Server::Root>
        + Send
        + Sync
        + 'static,
{
    type Out = ();
    
    #[instrument(skip_all)]
    fn apply(self, mut entity: prelude::EntityWorldMut) {
        let Self {
            game_init,
            player_init,
            reaction,
        } = self;

        match Server::init(game_init, player_init, reaction) {
            Ok(server) => {
                // TODO does this need to exist on the server? This can probably be removed, and Root can be moved to client only
                let root = common::component::Root::<Server::Common>::new(server.root().clone());
                let game_id = common::component::GameId::new(server.game_id());

                entity.insert((
                    prelude::Name::new(format!("[{}] spru server", game_id.friendly_display())),
                    game_id,
                    server::component::Runner::new(server),
                    root,
                ));

                entity.trigger(|entity| {
                    server::event::Initialized {
                        entity,
                        game_id,
                    }
                });
            }
            Err(error) => {
                entity.trigger(|entity| {
                    server::event::InitializeError {
                        entity,
                        error,
                    }
                });
            }
        }
    }
}

#[derive_where(Debug; <Server::Reaction as spru::Reaction>::Trigger)]
pub struct ManualTrigger<Server: server::ServerSSS> {
    pub trigger: <Server::Reaction as spru::Reaction>::Trigger,
}

impl<Server: server::ServerSSS> ManualTrigger<Server> {
    pub fn new(trigger: <Server::Reaction as spru::Reaction>::Trigger) -> Self {
        Self {
            trigger,
        }
    }
}

impl<Server: server::ServerSSS> prelude::EntityCommand for ManualTrigger<Server> 
where
    <Server::Reaction as spru::Reaction>::Trigger: Clone,
{
    type Out = prelude::Result;

    #[instrument(skip_all)]
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            trigger,
        } = self;

        let result = entity
            .get_components_mut::<&mut server::component::Runner<Server>>()?
            .server.manual_trigger(trigger.clone());
            
        match result {
            Ok(output) => {
                let spru::server::Output {
                    outbound,
                    events,
                    ret: (),
                } = output;

                entity.trigger(|entity| server::event::ManualTrigger::<Server> {
                    entity,
                    trigger,
                });

                entity.get_components_mut::<&mut server::component::ToClient<Server>>()?
                    .enqueue_outbound(outbound);

                server::trigger_events::<Server>(entity.id(), &mut entity, events);
            }
            Err(error) => {
                entity.trigger(|entity| server::event::ManualTriggerError::<Server> {
                    entity,
                    trigger,
                    error: Arc::new(error),
                });
            },
        };

        Ok(())
    }
}

#[derive(Debug)]
pub struct RemovePlayer<Server> {
    pub player_id: spru::player::Id,
    _p: PhantomData<Server>,
}

impl<Server> RemovePlayer<Server> {
    pub fn new(player_id: spru::player::Id) -> Self {
        Self {
            player_id,
            _p: PhantomData,
        }
    }
}

impl<Server: server::ServerSSS> prelude::EntityCommand for RemovePlayer<Server> {
    type Out = prelude::Result;

    #[instrument(skip_all)]
    fn apply(self, mut entity: prelude::EntityWorldMut) -> Self::Out {
        let Self {
            player_id,
            _p,
        } = self;

        let (mut runner, mut to_client) = entity.get_components_mut::<(
            &mut server::component::Runner<Server>, 
            &mut server::component::ToClient<Server>
        )>()?;

        match runner.server.remove_player(player_id) {
            Ok(output) => {
                let spru::server::Output {
                    outbound,
                    events,
                    ret: (),
                } = output;

                to_client.enqueue_outbound(outbound);

                entity.trigger(|entity| server::event::PlayerRemoved {
                    entity,
                    player_id,
                });

                server::trigger_events::<Server>(entity.id(), &mut entity, events);
            }
            Err(error) => {
                entity.trigger(|entity| server::event::PlayerRemoveError {
                    entity,
                    player_id,
                    error: Arc::new(error),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct DeactivatePlayer<Server> {
    pub player_id: spru::player::Id,
    _p: PhantomData<Server>,
}

impl<Server> DeactivatePlayer<Server> {
    pub fn new(player_id: spru::player::Id) -> Self {
        Self {
            player_id,
            _p: PhantomData,
        }
    }
}

impl<Server: server::ServerSSS> prelude::EntityCommand for DeactivatePlayer<Server> {
    type Out = prelude::Result;

    #[instrument(skip_all)]
    fn apply(self, mut entity: prelude::EntityWorldMut) -> Self::Out {
        let Self {
            player_id,
            _p,
        } = self;

        let (mut runner, mut to_client) = entity.get_components_mut::<(
            &mut server::component::Runner<Server>,
            &mut server::component::ToClient<Server>,
        )>()?;

        match runner.server.deactivate_player(player_id) {
            Ok(output) => {
                let spru::server::Output {
                    outbound,
                    events,
                    ret: (),
                } = output;

                to_client.enqueue_outbound(outbound);

                entity.trigger(|entity| server::event::PlayerDeactivated {
                    entity,
                    player_id,
                });

                server::trigger_events::<Server>(entity.id(), &mut entity, events);
            }
            Err(error) => {
                entity.trigger(|entity| server::event::PlayerDeactivateError {
                    entity,
                    player_id,
                    error: Arc::new(error),
                });
            }
        }

        Ok(())
    }
}
