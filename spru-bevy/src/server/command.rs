use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

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
    fn apply(self, mut entity: prelude::EntityWorldMut) {
        let Self {
            game_init,
            player_init,
            reaction,
        } = self;

        let result = (|| {
            let server = Server::init(game_init, player_init, reaction)?;
            let root = common::component::Root::<Server::Common>::new(server.root().clone());
            let game_id = common::component::GameId::new(server.game_id());

            entity.insert((
                prelude::Name::new(format!("[{}] spru server", game_id.friendly_display())),
                game_id,
                server::component::Runner::new(server),
                root,
            ));

            Ok(game_id)
        })();

        entity.trigger(|entity| server::event::Init::<Server> {
            entity,
            result,
            _server: PhantomData,
        });
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

impl<Server: server::ServerSSS> prelude::EntityCommand<prelude::Result> for ManualTrigger<Server> {
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> prelude::Result {
        let Self {
            trigger,
        } = self;

        let result = entity
            .get_components_mut::<&mut server::component::Runner<Server>>()?
            .server.manual_trigger(trigger);
            
        let result = match result {
            Ok(output) => {
                let spru::server::Output {
                    outbound,
                    events,
                    ret,
                } = output;

                entity.get_components_mut::<&mut server::component::ToClient<Server>>()?
                    .enqueue_outbound(outbound);

                server::trigger_events::<Server>(entity.id(), &mut entity, events);

                Ok(ret)
            }
            Err(err) => Err(err),
        };

        entity.trigger(|entity| server::event::ManualTrigger::<Server> {
            entity,
            result,
            _server: PhantomData,
        });

        Ok(())
    }
}

