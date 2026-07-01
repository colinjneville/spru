use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::client::remote;

#[derive_where(Debug, Default; )]
pub struct Plugin<Client: crate::client::ClientSSS> {
    _client: PhantomData<fn() -> Client>,
}

impl<Client> prelude::Plugin for Plugin<Client>
where
    Client: crate::client::ClientSSS<
        Action: serde::de::DeserializeOwned,
        Interaction: serde::Serialize,
        GameOutcome: serde::de::DeserializeOwned,
        Root: serde::de::DeserializeOwned,
        State: spru::State<Repr: serde::de::DeserializeOwned>,
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
                aeronet_webtransport::client::WebTransportClientPlugin,
            ))
            .add_systems(
                prelude::FixedUpdate,
                (
                    remote::system::seed_client::<Client>,
                    remote::system::propagate_remote_queues::<Client>,
                ),
            )
            .add_observer(remote::observer::on_connected::<Client>)
        ;
    }
}

