use bevy::{ecs::event::EntityEvent as _, prelude};

use crate::server::{self, remote};

pub fn on_opened(
    trigger: prelude::On<prelude::Add, aeronet::io::server::Server>, 
    q_servers: prelude::Query<&aeronet::io::connection::LocalAddr>,
) {
    
    let server = trigger.event_target();
    let local_addr = q_servers
        .get(server)
        .expect("Expected server LocalAddr");
    prelude::info!("{server} opened on {}", **local_addr);
}

pub fn on_session_request<Server: crate::server::ServerSSS>(
    mut request: prelude::On<aeronet_webtransport::server::SessionRequest>, 
    world: &mut prelude::World,
    // clients: prelude::Query<&prelude::ChildOf>
    q_clients: &mut prelude::QueryState<(
        &prelude::ChildOf,
    )>,
    q_server: &mut prelude::QueryState<(
        &mut crate::server::component::Runner<Server>,
        &mut crate::server::component::ToClient<Server>,
        &mut crate::server::component::PendingClients<Server>,
    )>,
) -> prelude::Result {
    let client = request.event_target();
    let q_clients = q_clients.query(world);
    
    let Ok((&prelude::ChildOf(server), )) = q_clients.get(client) else {
        return Ok(());
    };

    let response = {
        let mut event = remote::event::AttemptedConnection::<<Server::PlayerInit as spru::player::Init>::In> { 
            entity: client, 
            headers: request.headers.clone(),
            response: None,
        };
        
        world.trigger_ref(&mut event);

        let (mut runner, mut to_client, mut pending_clients, ) = q_server.get_mut(world, server)?;

        let response = event.response.unwrap_or(super::JoinRequestResponse::RejectNotFound);
        prelude::info!("{server} responding to connection request from {client} with: {response}");

        match response {
            super::JoinRequestResponse::AcceptNew(player_init_in) => {
                match runner.server.add_player(player_init_in) {
                    Ok(output) => {
                        let spru::server::Output {
                            outbound,
                            events,
                            ret,
                        } = output;
                        let player_id = ret.local_player_id();

                        to_client.enqueue_outbound(outbound);

                        pending_clients.enqueue(crate::server::PendingClient {
                            seed: ret,
                        });

                        world.commands().entity(client)
                            .insert(server::remote::component::RemoteClient { player_id });

                        aeronet_webtransport::server::SessionResponse::Accepted

                    },
                    Err(err) => {
                        prelude::info!("{server} rejected {client}: {err}");
                        aeronet_webtransport::server::SessionResponse::Forbidden
                    },
                }
            }
            super::JoinRequestResponse::AcceptReconnect(player_id) => todo!(),
            super::JoinRequestResponse::RejectNotFound => aeronet_webtransport::server::SessionResponse::NotFound,
            super::JoinRequestResponse::RejectNotAllowed => aeronet_webtransport::server::SessionResponse::Forbidden,
        }
    };

    request.respond(response);

    Ok(())
}

pub fn on_connected<Server: crate::server::ServerSSS>(
    trigger: prelude::On<prelude::Add, aeronet::io::Session>, 
    mut commands: prelude::Commands,
    clients: prelude::Query<(
        &prelude::ChildOf,
        &aeronet::io::Session,
    )>,
    // mut q_server: prelude::Query<(
    //     &mut crate::server::component::Runner<Server>,
    // )>,
) -> prelude::Result<()> {
    let client = trigger.event_target();
    let Ok((&prelude::ChildOf(server), session, )) = clients.get(client) else {
        return Ok(());
    };

    commands.entity(client)
        .insert(aeronet::transport::Transport::new(
            session,
            crate::remote::CLIENT_TO_SERVER_LANES,
            crate::remote::SERVER_TO_CLIENT_LANES,
            bevy::platform::time::Instant::now(),
        )?);

    prelude::info!("{client} connected to {server}");

    Ok(())
}