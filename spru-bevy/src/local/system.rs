use bevy::prelude;

pub fn propagate_local_queues<Server, Client>(
    q_server: prelude::Query<(
        &crate::common::component::GameId,
        &mut crate::server::component::FromClient<Server>,
        &mut crate::server::component::ToClient<Server>,
    )>,
    mut q_client: prelude::Query<(
        &crate::common::component::GameId,
        &mut crate::client::component::FromServer<Client>,
        &mut crate::client::component::ToServer<Client>,
        &crate::client::component::ClientId,
    )>,
)
where 
    Server: crate::server::ServerSSS,
    Client: crate::client::ClientSSS<
        Common = Server::Common,
    >,
{
    for (server_id, mut from_client, mut to_client) in q_server {
        for (client_server_id, mut from_server, mut to_server, client_player_id) in q_client.reborrow() {
            if server_id == client_server_id {
                while let Some(signal) = to_client.dequeue(client_player_id.0) {
                    from_server.enqueue(signal);
                }
                while let Some(signal) = to_server.dequeue() {
                    from_client.enqueue(client_player_id.0, signal);
                }
            }
        }
    }
}

pub fn create_local_clients<Server: crate::server::ServerSSS, Client: crate::client::ClientSSS>(
    mut commands: prelude::Commands,
    q_server: prelude::Query<(
        &crate::common::component::GameId,
        &mut crate::server::component::PendingClients<Server>,
    )>,
)
where 
    Server: crate::server::ServerSSS,
    Client: crate::client::ClientSSS<
        Common = Server::Common,
    >,
{
    for (&game_id, mut pending_clients) in q_server {
        while let Some(ret) = pending_clients.dequeue() {
            let spru::server::add_player::Ret {
                client_init,
                player_id: _player_id,
            } = ret;

            commands.queue(crate::client::command::SpawnClient::<Client> {
                init: client_init,
                game_id,
            });
        }
    }
}