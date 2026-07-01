use std::marker::PhantomData;

use bevy::{ecs::event::EntityEvent as _, prelude};

use crate::{common, server::{self, remote}};

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


// Needs to be public for on_session_request
#[doc(hidden)]
#[derive(Debug)]
pub struct AttemptedConnectionEventKey<Server: crate::server::ServerSSS>(bevy::ecs::event::EventKey, PhantomData<Server>);

impl<Server: crate::server::ServerSSS> prelude::FromWorld for AttemptedConnectionEventKey<Server> {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let key = world.register_event_key::<remote::event::AttemptedConnection::<<Server::PlayerInit as spru::player::Init>::In>>();
        Self(key, PhantomData)
    }
}

pub fn on_session_request<Server: crate::server::ServerSSS>(
    mut request: prelude::On<aeronet_webtransport::server::SessionRequest>, 
    mut commands: prelude::Commands,
    // clients: prelude::Query<&prelude::ChildOf>

    mut set: prelude::ParamSet<(
        bevy::ecs::world::DeferredWorld,
        prelude::Query<(
            &prelude::ChildOf,
        )>,
        prelude::Query<(
            &mut crate::server::component::Runner<Server>,
            &mut crate::server::component::ToClient<Server>,
        )>,
    )>,
    event_key: prelude::Local<AttemptedConnectionEventKey<Server>>,
) -> prelude::Result {
    let client = request.event_target();
    
    let Ok((&prelude::ChildOf(server), )) = set.p1().get(client) else {
        prelude::warn!("on_session_request is not child of a server");
        return Ok(());
    };

    let response = 'response: {
        // Attach our path to an arbitrary base url so we can parse the query string
        let localhost = url::Url::parse("https://localhost").unwrap();
        
        let url = match localhost.join(&request.path) {
            Ok(url) => url,
            Err(err) => {
                prelude::warn!("Attempted connection path '{}' is not a valid path: {err}", request.path);
                break 'response aeronet_webtransport::server::SessionResponse::NotFound;
            }
        };

        let mut event = remote::event::AttemptedConnection::<<Server::PlayerInit as spru::player::Init>::In> { 
            entity: client, 
            // WASM can't use actual headers (currently), so all headers are passed as query parameters
            // headers: request.headers.clone(),
            headers: url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect(),
            
            response: None,
        };
        
        // Here we need to immediately trigger a sub-event and get the result.
        // observers cannot take &mut World as a parameter, but we can take a DeferredWorld, which is (mostly)
        // what World::trigger_mut uses internally. We do need to keep an EventKey, which we can do as a Local param.
        // SAFETY: `event_key` Local stores the key with the same type as `event`
        unsafe {
            set.p0().trigger_raw(
                event_key.0, 
                &mut event, 
                &mut bevy::ecs::event::PropagateEntityTrigger::default(), 
                bevy::ecs::change_detection::MaybeLocation::caller(),
            );
        }

        let mut q_server = set.p2();
        let (mut runner, mut to_client, ) = q_server.get_mut(server)?;

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

                        commands.entity(client)
                            .insert((
                                server::remote::component::RemoteClient { player_id },
                                server::remote::component::PendingClient { seed: Some(ret) },
                            ))
                            ;

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

pub fn on_connecting<Server: crate::server::ServerSSS>(
    trigger: prelude::On<prelude::Add, aeronet::io::SessionEndpoint>, 
) -> prelude::Result {

    Ok(())
}

pub fn on_connected<Server: crate::server::ServerSSS>(
    trigger: prelude::On<prelude::Add, aeronet::io::Session>, 
    mut commands: prelude::Commands,
    clients: prelude::Query<(
        &prelude::ChildOf,
        &aeronet::io::Session,
    )>,
) -> prelude::Result {
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

pub fn on_disconnected<Server: crate::server::ServerSSS>(
    trigger: prelude::On<aeronet::io::connection::Disconnected>,
    q_client: prelude::Query<(
        &common::component::GameId,
        Option<&server::remote::component::PendingClient<Server::Common>>,
        Option<&server::remote::component::RemoteClient>,
    )>,
) -> prelude::Result {
    if let Ok((game_id, pending_client, remote_client)) = q_client.get(trigger.entity) {
        let client_id = pending_client
            .map(|pc| pc.seed.as_ref())
            .flatten()
            .map(|seed| seed.local_player_id())
            .or(remote_client.map(|rc| rc.player_id));

        let client_id_text = client_id.map(|id| id.to_string()).unwrap_or_else(|| "Unknown".to_string());

        let _span = prelude::error_span!("on_disconnect", game_id = %game_id.friendly_display(), client_id = client_id_text).entered();

        prelude::info!(reason = ?trigger.reason, "Client disconnected");
    }
    Ok(())
}