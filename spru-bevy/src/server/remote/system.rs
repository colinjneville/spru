use std::time;

use bevy::prelude;

use crate::{common, remote, server};

pub fn propagate_remote_queues<Server>(
    q_server: prelude::Query<(
        &common::component::GameId,
        &mut server::component::FromClient<Server>,
        &mut server::component::ToClient<Server>,
        Option<&mut server::component::PendingClients<Server>>,
    )>,
    mut q_transport: prelude::Query<(
        &common::component::GameId,
        &server::remote::component::RemoteClient,
        &mut aeronet::transport::Transport,
    )>
) where
    Server: server::ServerSSS<
        Action: serde::Serialize,
        Interaction: serde::de::DeserializeOwned,
        Reaction: spru::Reaction<GameOutcome: serde::Serialize>,
        Root: serde::Serialize,
        State: spru::State<Repr: serde::Serialize>,
    >,
{
    for (game_id, mut from_client, mut to_client, mut pending_clients) in q_server {
        for (session_game_id, remote_client, mut transport) in q_transport.reborrow() {
            let player_id = remote_client.player_id;
            
            if game_id == session_game_id {
                let _span = prelude::error_span!("server::propagate_remote_queues", %game_id, %player_id).entered();

                for msg in transport.recv.msgs.drain() {
                    match msg.lane {
                        remote::CLIENT_TO_SERVER_LANE_COORDINATION => {
                            // TODO
                            prelude::trace!("Received signal");
                        }
                        remote::CLIENT_TO_SERVER_LANE_SIGNAL => {
                            match rmp_serde::from_slice::<spru::common::signal::ToServer::<Server::Common>>(&*msg.payload) {
                                Ok(signal) => {
                                    from_client.enqueue(player_id, signal);
                                    
                                    prelude::trace!("Received signal");
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

                // ACKs are not used currently
                let _ = transport.recv.acks.drain();

                if let Some(pending_clients) = pending_clients.as_mut() {
                    while let Some(pending_client) = pending_clients.dequeue() {
                        let _span = prelude::error_span!("seed client", player_id = %pending_client.seed.local_player_id()).entered();

                        match rmp_serde::to_vec(&pending_client.seed) {
                            Ok(msg) => {
                                match transport.send.push(remote::SERVER_TO_CLIENT_LANE_COORDINATION, msg.into(), time::Instant::now()) {
                                    Ok(_key) => {
                                        prelude::info!("Seeding client");
                                    }
                                    Err(err) => {
                                        prelude::warn!("Seed could not be sent, session will be dropped: {err}");
                                    }
                                }
                            }
                            Err(err) => {
                                prelude::error!("Failed to serialize seed: {err}");
                            }
                        }
                    }
                }

                while let Some(signal) = to_client.dequeue(player_id) {
                    match rmp_serde::to_vec(&signal) {
                        Ok(msg) => {
                            match transport.send.push(remote::SERVER_TO_CLIENT_LANE_SIGNAL, msg.into(), time::Instant::now()) {
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