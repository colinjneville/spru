use bevy::prelude;

use crate::{client, remote};

pub(crate) fn seed_client<Client: client::ClientSSS>(
    mut commands: prelude::Commands,
    mut q_client: prelude::Query<(
        &mut client::component::FromServer<Client>,
    )>,
    q_transport: prelude::Query<(
        prelude::Entity,
        &mut aeronet::transport::Transport,
        &client::remote::component::PendingClient<Client>,
        &client::remote::component::RemoteFor,
    )>,
) 
    -> prelude::Result 
where
    Client: crate::client::ClientSSS<
        Action: serde::de::DeserializeOwned,
        Interaction: serde::Serialize,
        GameOutcome: serde::de::DeserializeOwned,
        Root: serde::de::DeserializeOwned,
        State: spru::State<Repr: serde::de::DeserializeOwned>,
    >,
{
    for (child, mut transport, _pending, remote_for) in q_transport {
        prelude::info_once!("q_transport");

        let parent = remote_for.remote_for();

        let (mut from_server, ) = q_client.get_mut(parent)
            .expect("PendingClient must be child of client entity");

        // aeronet forces us to drain the message queue every frame, even though we can't process the signals yet
        for msg in transport.recv.msgs.drain() {
            match msg.lane {
                remote::SERVER_TO_CLIENT_LANE_COORDINATION => {
                    commands.entity(child)
                        .remove::<client::remote::component::PendingClient<Client>>();

                    match remote::deserialize::<spru::common::Seed<Client::Common>>(&msg.payload) {
                        Ok(seed) => {
                            let _entered = prelude::error_span!("seed message", game_id = %seed.game_id(), player_id = %seed.local_player_id()).entered();
                            prelude::info!("Creating client");
                            commands
                                .entity(parent)
                                .queue(crate::client::command::Init::<Client> { seed });
                        }
                        Err(err) => {
                            prelude::warn!("Invalid seed: {err}");
                        }
                    }
                }
                remote::SERVER_TO_CLIENT_LANE_SIGNAL => {
                    enqueue_signal(&mut from_server, msg.payload)?;
                }
                lane @ _ => {
                    prelude::warn!("Invalid lane {lane}");
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn propagate_remote_queues<Client>(
    mut q_client: prelude::Query<(
        &mut client::component::FromServer<Client>,
        &mut client::component::ToServer<Client>,
        &client::remote::component::Remote,
    )>,
    mut q_transport: prelude::Query<(
        &mut aeronet::transport::Transport,
    )>,
    client_map: prelude::Res<client::resource::ClientMap>,
) 
    -> prelude::Result
where
    Client: crate::client::ClientSSS<
        Action: serde::de::DeserializeOwned,
        Interaction: serde::Serialize,
        GameOutcome: serde::de::DeserializeOwned,
        Root: serde::de::DeserializeOwned,
        State: spru::State<Repr: serde::de::DeserializeOwned>,
    >,
{
    for (game_id, client_id, client_entity) in client_map.iter() {
        if let Ok((mut from_server, mut to_server, remote)) = q_client.get_mut(client_entity) {
            if let Ok((mut transport, )) = q_transport.get_mut(remote.remote()) {
                let _span = prelude::error_span!("client::propagate_remote_queues", %game_id, %client_id).entered();
                for msg in transport.recv.msgs.drain() {
                    match msg.lane {
                        // COORDINATION is currently only used for initial seeding
                        // remote::SERVER_TO_CLIENT_LANE_COORDINATION => {
                            
                        // }
                        remote::SERVER_TO_CLIENT_LANE_SIGNAL => {
                            enqueue_signal(&mut from_server, msg.payload)?;
                        }
                        lane @ _ => {
                            prelude::warn!("Invalid lane {lane}");
                        }
                    }
                }

                while let Some(signal) = to_server.dequeue() {
                    match remote::serialize(&signal) {
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

    Ok(())
}

fn enqueue_signal<Client: client::ClientSSS>(
    from_server: &mut prelude::Mut<client::component::FromServer<Client>>, 
    payload: Vec<u8>,
) 
    -> prelude::Result 
where
    Client: crate::client::ClientSSS<
        Action: serde::de::DeserializeOwned,
        Interaction: serde::Serialize,
        GameOutcome: serde::de::DeserializeOwned,
        Root: serde::de::DeserializeOwned,
        State: spru::State<Repr: serde::de::DeserializeOwned>,
    >,
{
    match remote::deserialize::<spru::common::signal::ToClient::<Client::Common>>(&*payload) {
        Ok(signal) => {
            from_server.enqueue(signal);

            prelude::trace!("Sent signal");

            Ok(())
        }
        Err(err) => {
            prelude::warn!("Invalid signal: {err}");
            Err(err.into())
        }
    }
}