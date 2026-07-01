use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::server;

#[derive_where(Debug, Default; )]
pub struct Plugin<Server: crate::server::ServerSSS> {
    _server: PhantomData<fn() -> Server>,
}

impl<Server> prelude::Plugin for Plugin<Server>
where
    Server: crate::server::ServerSSS<
        Action: serde::Serialize,
        Reaction: spru::Reaction<GameOutcome: serde::Serialize>,
        Interaction: serde::de::DeserializeOwned,
        Root: serde::Serialize,
        State: spru::State<Repr: serde::Serialize>,
    >,
{
    fn build(&self, app: &mut prelude::App) {
        // Equivalent to aeronet::AeronetPlugins, but AeronetPlugins
        // does not check for existing plugins
        if !app.is_plugin_added::<aeronet::io::AeronetIoPlugin>() {
            app.add_plugins(aeronet::io::AeronetIoPlugin);
        }
        if !app.is_plugin_added::<aeronet::transport::AeronetTransportPlugin>() {
            app.add_plugins(aeronet::transport::AeronetTransportPlugin);
        }

        app
            .add_plugins((
                aeronet_webtransport::server::WebTransportServerPlugin,
            ))
            .add_systems(
                prelude::FixedUpdate,
                (
                    server::remote::system::seed_client::<Server>,
                    server::remote::system::propagate_remote_queues::<Server>,
                ),
            )
            .add_observer(server::remote::observer::on_opened)
            .add_observer(server::remote::observer::on_session_request::<Server>)
            .add_observer(server::remote::observer::on_connecting::<Server>)
            .add_observer(server::remote::observer::on_connected::<Server>)
            .add_observer(server::remote::observer::on_disconnected::<Server>)
        ;
    }
}
