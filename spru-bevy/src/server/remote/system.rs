use std::time;

use bevy::prelude;

use crate::{common, remote, server};

pub fn propagate_remote_queues<Server>(
    mut commands: prelude::Commands,
    q_server: prelude::Query<(
        &common::component::GameId,
        &mut server::component::FromClient<Server>,
        &mut server::component::ToClient<Server>,
    )>,
    mut q_transport: prelude::Query<(
        prelude::Entity,
        &common::component::GameId,
        &server::remote::component::RemoteClient,
        &mut aeronet::transport::Transport,
        Option<&mut server::remote::component::PendingRemote<Server::Common>>,
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
    // TODO convert to use Resource Maps
    for (game_id, mut from_client, mut to_client) in q_server {
        for (transport_entity, session_game_id, remote_client, mut transport, mut pending_remote) in q_transport.reborrow() {
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
                                    let message = format!("Invalid signal: {err}");
                                    prelude::warn!("{message}");
                                    commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
                                }
                            }
                        }
                        lane @ _ => {
                            let message = format!("Invalid lane {lane}");
                            prelude::warn!("{message}");
                            commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
                        }
                    }
                }

                // ACKs are not used currently
                let _ = transport.recv.acks.drain();

                if let Some(pending_remote) = &mut pending_remote {
                    if let Some(seed) = pending_remote.seed.take() {
                        commands.entity(transport_entity)
                            .remove::<server::remote::component::PendingRemote<Server::Common>>();

                        let _span = prelude::error_span!("seed client", player_id = %seed.local_player_id()).entered();

                        match rmp_serde::to_vec(&seed) {
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
                                // Most likely the server's application is buggy, but we will disconnect the client and maybe
                                // on reconnect they will avoid the bugged serialization
                                let message = format!("Failed to serialize seed: {err}");
                                prelude::error!("{message}");
                                commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
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
                            // Most likely the server's application is buggy, but we will disconnect the client and maybe
                            // on reconnect they will avoid the bugged serialization
                            let message = format!("Signal could not be serialized: {err}");
                            prelude::error!("{message}");
                            commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
                        }
                    }
                }
            }
        }
    }
}