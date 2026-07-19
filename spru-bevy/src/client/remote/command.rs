use std::marker::PhantomData;

use bevy::prelude;
use tracing::instrument;

use crate::{client, remote};

#[derive(Debug)]
pub struct JoinRemote<Client> {
    config: client::remote::component::ConnectionConfig,
    _p: PhantomData<Client>,
}

impl<Client> JoinRemote<Client> {
    pub fn new(config: client::remote::component::ConnectionConfig) -> Self {
        Self {
            config,
            _p: PhantomData,
        }
    }

    /// Add a key-value pair header entry
    /// Note: This is currently implemented using query parameters in the URL, as browser implementations of webtransport
    /// don't currently support adding to the request headers.
    /// This appears to be changing, and implementation may be able to switch to actual headers: 
    /// https://github.com/w3c/webtransport/issues/263
    #[must_use]
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.config.headers.insert(key, value);
        self
    }

    /// Add a key-value pair header entry
    /// Note: This is currently implemented using query parameters in the URL, as browser implementations of webtransport
    /// don't currently support adding to the request headers.
    /// This appears to be changing, and implementation may be able to switch to actual headers: 
    /// https://github.com/w3c/webtransport/issues/263
    pub fn set_header(&mut self, key: String, value: String) {
        self.config.headers.insert(key, value);
    }
}

impl<Client: client::ClientSSS> JoinRemote<Client> {
    #[instrument(skip_all)]
    fn on_connected(
        trigger: prelude::On<prelude::Add, aeronet::io::Session>, 
        mut commands: prelude::Commands,
        q_client: prelude::Query<(
            &aeronet::io::Session,
            &client::remote::component::RemoteFor,
        )>,
    ) -> prelude::Result {
        let remote_client_entity = trigger.entity;
        let Ok((session, remote_for)) = q_client.get(remote_client_entity) else {
            return Ok(());
        };
        let server_entity = remote_for.remote_for();

        commands.entity(remote_client_entity)
            .insert(aeronet::transport::Transport::new(
                session,
                crate::remote::SERVER_TO_CLIENT_LANES,
                crate::remote::CLIENT_TO_SERVER_LANES,
                bevy::platform::time::Instant::now(),
            )?);

        commands.entity(server_entity)
            .trigger(|entity| client::remote::event::Connected { 
                entity, 
            });

        prelude::info!("{remote_client_entity} connected to server");

        Ok(())
    }

    #[instrument(skip_all)]
    fn on_disconnected(
        disconnected: prelude::On<aeronet::io::connection::Disconnected>,
        mut commands: prelude::Commands,
        q_session: prelude::Query<(
            &client::remote::component::RemoteFor,
        )>,
    ) {
        let (remote_for, ) = q_session.get(disconnected.entity).expect("Expected RemoteFor");
        let client_entity = remote_for.remote_for();

        prelude::info!("Forwarding aeronet Disconnected as spru_bevy Disconnected");

        commands
            .entity(client_entity)
            .trigger(|entity| {
                client::remote::event::Disconnected {
                    entity,
                    reason: remote::DisconnectedReason::from_aeronet(&disconnected.reason),
                }
            })
            ;
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand for JoinRemote<Client> {
    type Out = ();
    
    #[instrument(skip_all)]
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            config,
            _p,
        } = self;

        let url = config.url();
        let client_config = config.client_config();

        prelude::info!("Requesting connection to server at {url}");
        let client_entity = entity.id();
        entity
            .insert((
                client::component::FromServer::<Client>::default(),
                config,
            ))
            ;

        // Keep aeronet on a separate child entity as aeronet will despawn it on disconnect
        entity.world_scope(|world| {
            let mut remote_entity = world
                .spawn((
                    client::remote::component::PendingClient::<Client>::default(),
                    prelude::ChildOf(client_entity),
                    client::remote::component::RemoteFor(client_entity),
                ));

            remote_entity
                .observe(Self::on_connected)
                .observe(Self::on_disconnected)
                ;

            aeronet_webtransport::client::WebTransportClient::connect(client_config, url)
                .apply(remote_entity);
        });

    }
}

/// Trigger a disconnect from the server.
#[derive(Debug)]
pub struct Disconnect<Client> {
    pub reason: String,
    _p: PhantomData<Client>,
}

impl<Client> Disconnect<Client> {
    pub fn new(reason: String) -> Self {
        Self {
            reason,
            _p: PhantomData,
        }
    }
}

impl<Client: client::ClientSSS> prelude::EntityCommand for Disconnect<Client> {
    type Out = ();

    #[instrument(skip_all)]
    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            reason,
            _p,
        } = self;

        if let Ok((remote, )) = entity.get_components::<(&client::remote::component::Remote, )>() {
            let remote_entity = remote.remote();
            entity.world_scope(|world| {
                world
                    .entity_mut(remote_entity)
                    .trigger(|entity| aeronet::io::connection::Disconnect {
                        entity,
                        reason,
                    })
                    ;
            });
        }
    }
}
