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
) where
    Server: crate::server::ServerSSS,
    Client: crate::client::ClientSSS<Common = Server::Common>,
{
    for (game_id, mut from_client, mut to_client) in q_server {
        for (client_game_id, mut from_server, mut to_server, client_player_id) in
            q_client.reborrow()
        {
            if game_id == client_game_id {
                let mut count = 0;
                while let Some(signal) = to_client.dequeue(**client_player_id) {
                    from_server.enqueue(signal);
                    count += 1;
                }
                if count > 0 {
                    prelude::trace!(
                        "[{game_id}] Transfered {count} signals to local client {client_player_id}"
                    );
                }

                let mut count = 0;
                while let Some(signal) = to_server.dequeue() {
                    from_client.enqueue(**client_player_id, signal);
                    count += 1;
                }

                if count > 0 {
                    prelude::trace!(
                        "[{game_id}] Transfered {count} signals from local client {client_player_id}"
                    );
                }
            }
        }
    }
}
