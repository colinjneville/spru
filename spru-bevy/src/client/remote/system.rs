use bevy::prelude;

use crate::{client, common, remote};


pub fn propagate_remote_queues<Client>(
    mut commands: prelude::Commands,
    q_client: prelude::Query<(
        &common::component::GameId,
        &mut client::component::FromServer<Client>,
        &mut client::component::ToServer<Client>,
        &client::component::ClientId,
    )>,
    mut q_transport: prelude::Query<(
        &crate::common::component::GameId,
        &crate::client::component::ClientId,
        &mut aeronet::transport::Transport,
    )>,
) where
    Client: crate::client::ClientSSS<
        Action: serde::de::DeserializeOwned,
        Interaction: serde::Serialize,
        GameOutcome: serde::de::DeserializeOwned,
        Root: serde::de::DeserializeOwned,
        State: spru::State<Repr: serde::de::DeserializeOwned>,
    >,
{
    for (game_id, mut from_server, mut to_server, player_id) in q_client {
        for (session_game_id, session_player_id, mut transport) in q_transport.reborrow() {
            if game_id == session_game_id && player_id == session_player_id {
                let _span = prelude::error_span!("client::propagate_remote_queues", %game_id, %player_id).entered();

                for msg in transport.recv.msgs.drain() {
                    match msg.lane {
                        remote::SERVER_TO_CLIENT_LANE_COORDINATION => {
                            match rmp_serde::from_slice::<spru::common::Seed<Client::Common>>(&msg.payload) {
                                Ok(seed) => {
                                    prelude::info!("Creating client");
                                    commands
                                        .spawn_empty()
                                        .queue(crate::client::command::Init::<Client> { seed });
                                }
                                Err(err) => {
                                    prelude::warn!("Invalid seed: {err}");
                                }
                            }
                        }
                        remote::SERVER_TO_CLIENT_LANE_SIGNAL => {
                            match rmp_serde::from_slice::<spru::common::signal::ToClient::<Client::Common>>(&*msg.payload) {
                                Ok(signal) => {
                                    from_server.enqueue(signal);

                                    prelude::trace!("Sent signal");
                                }
                                Err(err) => {
                                    prelude::warn!("Invalid signal: {err}");
                                }
                            }
                        }
                        lane @ _ => {
                            prelude::warn!("Invalid lane {lane}");
                        }
                    }
                }

                while let Some(signal) = to_server.dequeue() {
                    match rmp_serde::to_vec(&signal) {
                        Ok(msg) => {
                            match transport.send.push(remote::CLIENT_TO_SERVER_LANE_SIGNAL, msg.into(), bevy::platform::time::Instant::now()) {
                                Ok(_key) => {
                                    prelude::trace!("Sent signal");
                                }
                                Err(err) => {
                                    prelude::warn!("Signal could not be sent, session will be dropped: {err}");
                                }
                            }
                        }
                        Err(err) => {
                            prelude::error!("Signal could not be serialized: {err}");
                        }
                    }
                }
            }
        }
    }
}