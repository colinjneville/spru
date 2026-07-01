use std::time;

use bevy::prelude;

use crate::{remote, server};

pub fn seed_client<Server>(
    mut commands: prelude::Commands,
    q_pending_client: prelude::Query<(
        prelude::Entity,
        &mut server::remote::component::PendingClient<Server::Common>,
        &mut aeronet::transport::Transport,
    )>,
) 
    -> prelude::Result 
where 
    Server: server::ServerSSS<
        Action: serde::Serialize,
        Interaction: serde::de::DeserializeOwned,
        Reaction: spru::Reaction<GameOutcome: serde::Serialize>,
        Root: serde::Serialize,
        State: spru::State<Repr: serde::Serialize>,
    >,
{
    for (entity, mut pending_client, mut transport) in q_pending_client {
        commands.entity(entity)
            .remove::<server::remote::component::PendingClient<Server::Common>>();
        if let Some(seed) = pending_client.seed.take() {
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
                    commands.trigger(aeronet::io::connection::Disconnect::new(entity, message));
                }
            }
        }
    }
    
    Ok(())
}

pub fn propagate_remote_queues<Server>(
    mut commands: prelude::Commands,
    mut q_server: prelude::Query<(
        &mut server::component::FromClient<Server>,
        &mut server::component::ToClient<Server>,
        &prelude::Children,
    )>,
    mut q_transport: prelude::Query<(
        prelude::Entity,
        &server::remote::component::RemoteClient,
        &mut aeronet::transport::Transport,
    )>,
    server_map: prelude::Res<server::resource::ServerMap>,
) where
    Server: server::ServerSSS<
        Action: serde::Serialize,
        Interaction: serde::de::DeserializeOwned,
        Reaction: spru::Reaction<GameOutcome: serde::Serialize>,
        Root: serde::Serialize,
        State: spru::State<Repr: serde::Serialize>,
    >,
{
    for (game_id, server_entity) in server_map.iter() {
        if let Ok((mut from_client, mut to_client, children)) = q_server.get_mut(server_entity) {
            let mut child_iter = q_transport.iter_many_mut(children);
            while let Some((transport_entity, remote_client, mut transport)) = child_iter.fetch_next() {
                let player_id = remote_client.player_id;
                let _span = prelude::error_span!("server::propagate_remote_queues", %game_id, %player_id).entered();

                for msg in transport.recv.msgs.drain() {
                    match msg.lane {
                        // Not currently used except for intial seeding
                        // remote::CLIENT_TO_SERVER_LANE_COORDINATION => {
                        //     // TODO
                        //     prelude::trace!("Received signal");
                        // }
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

                while let Some(signal) = to_client.dequeue(player_id) {
                    match rmp_serde::to_vec(&signal) {
                        Ok(payload) => {
                            match transport.send.push(remote::SERVER_TO_CLIENT_LANE_SIGNAL, payload.into(), time::Instant::now()) {
                                Ok(_key) => { }
                                Err(err) => {
                                    let message = format!("Failed to send signal: {err}");
                                    prelude::error!("{message}");
                                    commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
                                }
                            }
                        }
                        Err(err) => {
                            let message = format!("Failed to serialize signal: {err}");
                            prelude::error!("{message}");
                            commands.trigger(aeronet::io::connection::Disconnect::new(transport_entity, message));
                        }
                    }
                }
            }
        }
    }
}