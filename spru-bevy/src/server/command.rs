use bevy::{ecs::system::RunSystemOnce, prelude};
use derive_where::derive_where;

#[derive(Debug)]
pub struct SpawnServer<Server: super::ServerSSS, GameInit> {
    pub game_init: GameInit, 
    pub player_init: Server::PlayerInit, 
    pub reaction: Server::Reaction,
}

impl<Server, GameInit> prelude::Command<prelude::Result<(prelude::Entity, crate::common::component::GameId)>> for SpawnServer<Server, GameInit> 
where
    Server: super::ServerSSS,
    GameInit: spru::game::Init<State = Server::State, Action = Server::Action, Root = Server::Root> + Send + Sync + 'static,
{
    fn apply(self, world: &mut bevy::ecs::world::World) -> prelude::Result<(prelude::Entity, crate::common::component::GameId)> {
        let Self {
            game_init,
            player_init,
            reaction,
        } = self;

        let runner = super::component::Runner::<Server>::new(game_init, player_init, reaction)?;

        let game_id = crate::common::component::GameId::default();

        let entity = world.spawn((
            game_id.clone(),
            runner,
        )).id();

        Ok((
            entity,
            game_id,
        ))
    }
}

#[derive_where(Debug; <Server::PlayerInit as spru::player::Init>::In)]
pub struct AddPlayer<Server: super::ServerSSS> {
    pub game_id: crate::common::component::GameId,
    pub player_init_input: <Server::PlayerInit as spru::player::Init>::In,
}

impl<Server> prelude::Command<prelude::Result<spru::player::Id>> for AddPlayer<Server> 
where
    Server: super::ServerSSS,
{
    fn apply(self, world: &mut bevy::ecs::world::World) -> prelude::Result<spru::player::Id> {
        let Self {
            game_id,
            player_init_input,
        } = self;

        if let Some(server_entity) = world.run_system_once_with(super::system::find_server::<Server>, game_id)
            .expect("System must be valid") 
        {
            let server_entity: prelude::Entity = server_entity;
            if let Ok((mut runner, mut to_client, mut pending_clients)) = world.query::<(
                &mut super::component::Runner<Server>,
                &mut super::component::ToClient<Server>,
                &mut super::component::PendingClients<Server>,
            )>().get_mut(world, server_entity) {
                let arg = spru::server::add_player::Arg {
                    init_input: player_init_input,
                };

                let spru::server::Output {
                    outbound,
                    events,
                    ret,
                } = runner.server.add_player(arg)?;

                let player_id = ret.player_id;

                pending_clients.enqueue(ret);

                for (client_id, signal) in outbound {
                    to_client.enqueue(client_id, signal);
                }

                return Ok(player_id);
            }
        }

        Err("Server not found".into())
    }
}
