use std::collections::HashMap;
use std::{marker::PhantomData, sync::Arc};
use std::time;

use bevy::prelude;

use tracing::{field::{Empty, display}, instrument};

use crate::{common, remote, server};

/// Constructs a [StartListener] command. 
#[derive(Debug, Default)]
pub struct StartListenerBuilder {
    /// The certificate and private key of the server.  
    /// webtransport requires the client to validate the certificate for
    /// any server it connects to. This can be a [self-signed](crate::remote::wtransport::Identity::self_signed)
    /// certificate, but the client must explicitly allow the self-signed certificate's hash
    /// (since it won't be validated by any root certificates). This means the hash must be 
    /// communicated out-of-band to the client along with the IP and port.  
    /// 
    /// If an identity is not provided, defaults to `localhost`
    /// and only local connections are possible.
    pub identity: Option<crate::remote::aeronet_webtransport::wtransport::Identity>,
    /// Which UDP port to listen on. Should be in the range 49152–65535.  
    /// 
    /// If not specified, a default port value will be used.
    pub port: Option<u16>,
    /// How frequently keep-alive messages will be sent in the absence of other messages.  
    /// 
    /// Defaults to no keep-alive messages.
    pub keep_alive_interval: Option<time::Duration>,
    /// How long the connection will remain open without messages.
    /// 
    /// Defaults to no timeout.
    pub max_idle_timeout: Option<time::Duration>,
}

impl StartListenerBuilder {
    /// A new [StartListener] command builder. Defaults to localhost:52152
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the [StartListener] command
    #[must_use]
    pub fn build<Server>(self) -> StartListener<Server> {
        let Self {
            identity,
            port,
            keep_alive_interval,
            max_idle_timeout,
        } = self;
        let identity = identity.unwrap_or_else(Self::localhost_identity);
        let port = port.unwrap_or(52152);

        let certificate = server::remote::component::Certificate::new(identity.certificate_chain().clone());

        let config = aeronet_webtransport::wtransport::ServerConfig::builder()
            .with_bind_default(port)
            .with_identity(identity)
            .keep_alive_interval(keep_alive_interval)
            .max_idle_timeout(max_idle_timeout)
            .expect("Max timeout should be before the heat death of the universe")
            .build();

        StartListener {
            config,
            certificate,
            _p: PhantomData,
        }
    }

    fn localhost_identity() -> aeronet_webtransport::wtransport::Identity {
        aeronet_webtransport::wtransport::Identity::self_signed(["localhost", "127.0.0.1", "::1"])
            .unwrap()
    }
}

/// Start listening for remote connections on the target server entity
#[derive(Debug)]
pub struct StartListener<Server> {
    config: aeronet_webtransport::server::ServerConfig,
    certificate: server::remote::component::Certificate,
    _p: PhantomData<Server>,
}

impl<Server: server::ServerSSS> StartListener<Server> {
    #[instrument(skip_all)]
    pub(crate) fn on_session_request(
        mut request: prelude::On<aeronet_webtransport::server::SessionRequest>, 
        mut commands: prelude::Commands,

        mut set: prelude::ParamSet<(
            bevy::ecs::world::DeferredWorld,
            prelude::Query<(
                &prelude::ChildOf,
            )>,
            prelude::Query<(
                &common::component::GameId,
            )>,
            prelude::Query<(
                &mut server::component::Runner<Server>,
                &mut server::component::ToClient<Server>,
            )>,
            prelude::Query<(
                &server::remote::component::ListenerFor,
            )>,
        )>,
        event_key: prelude::Local<AttemptedConnectionEventKey<Server>>,
    ) -> prelude::Result {
        let span = prelude::error_span!("on_session_request", remote_addr = %request.remote_addr, game_id = Empty).entered();
        let client_entity = request.entity;
        commands.entity(client_entity)
            .observe(Self::on_connected)
            .observe(Self::on_disconnected)
            ;

        prelude::info!("Received session request");
        
        let Ok((&prelude::ChildOf(listener_entity), )) = set.p1().get(client_entity) else {
            prelude::error!("Remote client is not the child of a listener");
            return Ok(());
        };

        let Ok((&server::remote::component::ListenerFor(server_entity), )) = set.p4().get(listener_entity) else {
            prelude::error!("Server listener is not attached to a server");
            return Ok(());
        };
        
        let (&game_id, ) = set.p2().get(server_entity)
            .expect("Server must have game id");
        let game_id = *game_id;

        span.record("game_id", display(game_id));

        enum Reason {
            NotFound,
            NotAllowed,
        }

        enum Response<Common: common::CommonSSS> {
            AcceptNew {
                seed: spru::common::Seed<Common>,
                headers: HashMap<String, String>,
            },
            AcceptReconnect {
                seed: spru::common::Seed<Common>,
                headers: HashMap<String, String>,
            },
            Reject {
                reason: Reason,
                message: String,
                headers: HashMap<String, String>,
            }
        }

        let response: Response<Server::Common> = 'response: {
            // Attach our path to an arbitrary base url so we can parse the query string
            let localhost = url::Url::parse("https://localhost").unwrap();
            
            let url = match localhost.join(&request.path) {
                Ok(url) => url,
                Err(err) => {
                    break 'response Response::Reject { 
                        reason: Reason::NotFound,
                        message: format!("Attempted connection path '{}' is not a valid path: {err}", request.path),
                        headers: HashMap::new(),
                    };
                }
            };

            let mut event = server::remote::event::ConnectionAttempted::<<Server::PlayerInit as spru::player::Init>::In> { 
                server_entity, 
                client_entity,
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
                    &mut bevy::ecs::event::EntityTrigger::default(), 
                    bevy::ecs::change_detection::MaybeLocation::caller(),
                );
            }

            let mut q_server = set.p3();
            let (mut runner, mut to_client, ) = q_server.get_mut(server_entity)?;

            let response = event.response
                .unwrap_or_else(|| super::JoinRequestResponse::RejectNotFound("No handler".to_string()));
            prelude::info!("Responding to connection request with: {response}");

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

                            commands
                                .entity(server_entity)
                                .trigger(|entity| server::event::PlayerAdded {
                                    entity,
                                    player_id,
                                })
                                .trigger(|server_entity| server::remote::event::RemotePlayerAdded {
                                    server_entity,
                                    client_entity,
                                    player_id,
                                });

                            server::trigger_events::<Server>(server_entity, &mut commands, events);

                            Response::AcceptNew { 
                                seed: ret, 
                                headers: event.headers,
                            }

                        },
                        Err(error) => {
                            let message = format!("{server_entity} rejected {client_entity}: {error}");

                            let error = Arc::new(error);

                            commands
                                .entity(server_entity)
                                .trigger(|entity| server::event::PlayerAddError {
                                    entity,
                                    error: error.clone(),
                                })
                                .trigger(|server_entity| server::remote::event::RemotePlayerAddError {
                                    server_entity,
                                    client_entity,
                                    error,
                                });

                            Response::Reject { 
                                reason: Reason::NotAllowed, 
                                message,
                                headers: event.headers,
                            }
                        },
                    }
                }
                super::JoinRequestResponse::AcceptReconnect(player_id) => {
                    prelude::info!("Accepting reconnect as {player_id}");
                    
                    match runner.server.reseed_player(player_id) {
                        Ok(output) => {
                            let spru::server::Output {
                                outbound,
                                events,
                                ret: seed,
                            } = output;
                            let player_id = seed.local_player_id();

                            to_client.enqueue_outbound(outbound);

                            commands
                                .entity(server_entity)
                                .trigger(|entity| server::event::PlayerReseeded {
                                    entity,
                                    player_id,
                                })
                                .trigger(|server_entity| server::remote::event::RemotePlayerReseeded {
                                    server_entity,
                                    client_entity,
                                    player_id,
                                });

                            server::trigger_events::<Server>(server_entity, &mut commands, events);

                            Response::AcceptReconnect { 
                                seed, 
                                headers: event.headers,
                            }
                        },
                        Err(error) => {
                            let message = format!("{server_entity} rejected {client_entity}: {error}");

                            let error = Arc::new(error);

                            commands
                                .entity(server_entity)
                                .trigger(|entity| server::event::PlayerReseedError {
                                    entity,
                                    player_id,
                                    error: error.clone(),
                                })
                                .trigger(|server_entity| server::remote::event::RemotePlayerReseedError {
                                    server_entity,
                                    client_entity,
                                    error,
                                });

                            Response::Reject { 
                                reason: Reason::NotAllowed, 
                                message,
                                headers: event.headers,
                            }
                        },
                    }
                }
                super::JoinRequestResponse::RejectNotFound(msg) => {
                    Response::Reject { 
                        reason: Reason::NotFound, 
                        message: format!("Rejecting session request with 404: {msg}"), 
                        headers: event.headers,
                    }
                }
                super::JoinRequestResponse::RejectNotAllowed(msg) => {
                    Response::Reject { 
                        reason: Reason::NotAllowed, 
                        message: format!("Rejecting session request with 403: {msg}"), 
                        headers: event.headers,
                    }
                }
            }
        };

        let response = match response {
            Response::AcceptNew { seed, headers } => {
                let player_id = seed.local_player_id();
                commands.entity(client_entity)
                    .insert((
                        common::component::GameId::new(game_id),
                        server::remote::component::RemoteClient { player_id },
                        server::remote::component::RemoteClientFor(server_entity),
                        server::remote::component::PendingClient { seed: Some(seed) },
                    ));

                commands.entity(server_entity)
                    .trigger(|server_entity| server::remote::event::ConnectionAccepted {
                        server_entity,
                        client_entity,
                        player_id,
                        headers,
                    });
                    
                aeronet_webtransport::server::SessionResponse::Accepted
            },
            Response::AcceptReconnect { seed, headers } => {
                let player_id = seed.local_player_id();
                commands.entity(client_entity)
                    .insert((
                        common::component::GameId::new(game_id),
                        server::remote::component::RemoteClient { player_id },
                        server::remote::component::RemoteClientFor(server_entity),
                        server::remote::component::PendingClient { seed: Some(seed) },
                    ));
                    
                commands.entity(server_entity)
                    .trigger(|server_entity| server::remote::event::ReconnectionAccepted {
                        server_entity,
                        client_entity,
                        player_id,
                        headers,
                    })
                ;
                aeronet_webtransport::server::SessionResponse::Accepted
            },
            Response::Reject { reason, message, headers } => {
                commands.entity(server_entity)
                    .trigger(|server_entity| server::remote::event::ConnectionRejected {
                        server_entity,
                        client_entity,
                        headers,
                        message,
                    })
                ;
                match reason {
                    Reason::NotFound => aeronet_webtransport::server::SessionResponse::NotFound,
                    Reason::NotAllowed => aeronet_webtransport::server::SessionResponse::Forbidden,
                }
            },
        };

        request.respond(response);

        Ok(())
    }

    #[instrument(skip_all)]
    fn on_opened(
        trigger: prelude::On<prelude::Add, aeronet::io::server::Server>, 
        q_servers: prelude::Query<(
            &common::component::GameId,
            &aeronet::io::connection::LocalAddr,
        )>,
    ) {
        
        let server_entity = trigger.entity;
        let _span = if let Ok((&server_id, local_addr)) = q_servers
            .get(server_entity)
        {
            Some(prelude::error_span!("on_opened", server_id = %server_id, local_addr = %local_addr.0).entered())
        } else {
            None
        };

        prelude::info!("webtransport server opened");
    }

    #[instrument(skip_all)]
    fn on_connected(
        trigger: prelude::On<prelude::Add, aeronet::io::Session>, 
        mut commands: prelude::Commands,
        clients: prelude::Query<(
            &server::remote::component::RemoteClientFor,
            &aeronet::io::Session,
        )>,
    ) -> prelude::Result {
        let client_entity = trigger.entity;
        let Ok((&server::remote::component::RemoteClientFor(server_entity), session, )) = clients.get(client_entity) else {
            return Ok(());
        };

        commands.entity(client_entity)
            .insert(aeronet::transport::Transport::new(
                session,
                crate::remote::CLIENT_TO_SERVER_LANES,
                crate::remote::SERVER_TO_CLIENT_LANES,
                bevy::platform::time::Instant::now(),
            )?);

        prelude::info!("{client_entity} connected to {server_entity}");

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) fn on_disconnected(
        trigger: prelude::On<aeronet::io::connection::Disconnected>,
        mut commands: prelude::Commands,
        q_client: prelude::Query<(
            &common::component::GameId,
            &server::remote::component::RemoteClient,
            &server::remote::component::RemoteClientFor,
        )>,
    ) -> prelude::Result {
        let remote_client_entity = trigger.entity;
        let (game_id, remote_client, remote_client_for) = q_client.get(remote_client_entity)?;
        let player_id = remote_client.player_id;
        let server_entity = remote_client_for.remote_client_for();

        let _span = prelude::error_span!("on_disconnect", game_id = %game_id.friendly_display(), %player_id).entered();

        prelude::info!(reason = ?trigger.reason, "Client disconnected");

        commands
            .entity(server_entity)
            .queue(server::command::DeactivatePlayer::<Server>::new(player_id))
            .trigger(|server_entity| {
                server::remote::event::RemotePlayerDisconnected {
                    server_entity,
                    client_entity: remote_client_entity,
                    player_id,
                    reason: remote::DisconnectedReason::from_aeronet(&trigger.reason),
                }
            })
            ;
        
        Ok(())
    }

    #[instrument(skip_all)]
    fn on_player_removed(
        player_removed: prelude::On<server::event::PlayerRemoved>,
        mut commands: prelude::Commands,
        q_server: prelude::Query<(
            &server::remote::component::RemoteClients,
        )>,
        q_session: prelude::Query<(
            &server::remote::component::RemoteClient,
        )>,
    ) {
        let server_entity = player_removed.entity;
        let Ok((remote_clients, )) = q_server.get(server_entity) else { return };

        let player_id = player_removed.player_id;

        // If this removed player was remote, disconnect the connection
        for &client_entity in remote_clients.remote_clients() {
            if let Ok((remote_client, )) = q_session.get(client_entity) {
                if remote_client.player_id == player_id {
                    commands
                        .entity(server_entity)
                        .trigger(|server_entity| server::remote::event::RemotePlayerRemoved {
                            server_entity,
                            client_entity,
                            player_id,
                        })
                        ;
                }
            }
        }
    }

    #[instrument(skip_all)]
    fn on_remote_player_removed(
        player_removed: prelude::On<server::remote::event::RemotePlayerRemoved>,
        mut commands: prelude::Commands,
    ) {
        commands.trigger(aeronet::io::connection::Disconnect::new(player_removed.client_entity, "Player was removed from the game"));
    }

    #[instrument(skip_all)]
    fn on_player_remove_error(
        player_remove_error: prelude::On<server::event::PlayerRemoveError>,
        mut commands: prelude::Commands,
        q_server: prelude::Query<(
            &server::remote::component::RemoteClients,
        )>,
        q_session: prelude::Query<(
            &server::remote::component::RemoteClient,
        )>,
    ) {
        let server_entity = player_remove_error.entity;
        let Ok((remote_clients, )) = q_server.get(server_entity) else { return };

        let player_id = player_remove_error.player_id;

        // If this removed player was remote, disconnect the connection
        for &client_entity in remote_clients.remote_clients() {
            if let Ok((remote_client, )) = q_session.get(client_entity) {
                if remote_client.player_id == player_id {
                    commands
                        .entity(server_entity)
                        .trigger(|server_entity| server::remote::event::RemotePlayerRemoveError {
                            server_entity,
                            client_entity,
                            player_id,
                            error: player_remove_error.error.clone(),
                        })
                        ;
                }
            }
        }
    }
}

impl<Server: server::ServerSSS> prelude::EntityCommand for StartListener<Server> {
    type Out = ();
    
    #[instrument(skip_all)]
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) {
        let Self {
            config,
            certificate,
            _p,
        } = self;

        let certificate_hash = certificate.hash();
        let spki_fingerprint = certificate.spki_fingerprint();

        if entity.contains::<server::component::Runner<Server>>() {
            let server_entity = entity.id();

            entity
                .insert((
                    certificate,
                    certificate_hash,
                    spki_fingerprint,
                ))
                .observe(Self::on_player_removed)
                .observe(Self::on_remote_player_removed)
                .observe(Self::on_player_remove_error)
            ;

            entity.world_scope(|world| {
                let mut listener_entity = world
                    .spawn((
                        prelude::ChildOf(server_entity),
                        server::remote::component::ListenerFor(server_entity),
                    ));

                listener_entity
                    // on_session_request has to be a global observer, for now at least
                    // .observe(Self::on_session_request)
                    .observe(Self::on_opened)
                    ;

                aeronet_webtransport::server::WebTransportServer::open(config)
                    .apply(listener_entity);
            });
            
        } else {
            prelude::error!("Cannot StartListener on an Entity without spru_bevy::server::component::Runner");
        }
    }
}

#[derive(Debug)]
pub(crate) struct AttemptedConnectionEventKey<Server: crate::server::ServerSSS>(bevy::ecs::event::EventKey, PhantomData<Server>);

impl<Server: crate::server::ServerSSS> prelude::FromWorld for AttemptedConnectionEventKey<Server> {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let key = world.register_event_key::<server::remote::event::ConnectionAttempted::<<Server::PlayerInit as spru::player::Init>::In>>();
        Self(key, PhantomData)
    }
}