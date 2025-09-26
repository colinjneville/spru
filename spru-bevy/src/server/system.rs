use bevy::prelude;

use crate::server::component;

pub fn run_server<Server: crate::server::ServerSSS>(
    mut q_server: prelude::Query<(
        &mut component::Runner<Server>, 
        &mut component::FromClient<Server>,
        &mut component::ToClient<Server>,
    )>,
)
    -> spru::server::signal::Result<()>
{
    // TODO this should probably be done as async-compute since we don't touch bevy from the server:
    // https://bevy-cheatbook.github.io/fundamentals/async-compute.html
    for (mut runner, mut from_client, mut to_client) in &mut q_server {
        while let Some((sender, signal)) = from_client.dequeue_any() {
            let out = runner.server.apply_signal(sender, signal)?;
            let spru::server::Output {
                outbound,
                events,
                ret,
            } = out;
            
            for (player_id, client_signal) in outbound {
                to_client.enqueue(player_id, client_signal);
            }
        }
    }

    Ok(())
}

pub fn add_player<Server: super::ServerSSS>(
    prelude::In((game_id, player_init_input)): prelude::In<(
        crate::common::component::GameId, 
        <Server::PlayerInit as spru::player::Init>::In)
    >,
    q_server: prelude::Query<(
        &crate::common::component::GameId,
        &mut super::component::Runner<Server>,
    )>,
)
    -> spru::server::add_player::Result<spru::client::init::Arg<Server::State, Server::Action, Server::Root>>
{
    // for (&server_game_id, runner) in q_server {
    //     if game_id == server_game_id 
    // }
    panic!()
}

pub(crate) fn find_server<'world, Server: super::ServerSSS>(
    prelude::In(game_id): prelude::In<crate::common::component::GameId>,
    q_server: prelude::Query<(
        prelude::Entity,
        &crate::common::component::GameId,
    )>,
)
    -> Option<prelude::Entity>
{
    for (entity, server_game_id) in q_server {
        if &game_id == server_game_id {
            return Some(entity);
        }
    }
    None
}