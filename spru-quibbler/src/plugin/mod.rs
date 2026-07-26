mod core;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "join")]
mod join;

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "hotseat")]
mod hotseat;

#[cfg(feature = "local")]
mod local;

#[cfg(feature = "remote")]
mod remote;

#[cfg(feature = "server")]
mod server;

#[cfg(feature = "ui")]
mod ui;

use bevy::prelude;

pub struct Group;

impl prelude::PluginGroup for Group {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder = bevy::app::PluginGroupBuilder::start::<Self>()
            .add(core::Core);

        #[cfg(feature = "client")]
        let builder = builder.add(client::Client);

        #[cfg(feature = "join")]
        let builder = builder.add(join::Join);

        #[cfg(feature = "host")]
        let builder = builder.add(host::Host);

        #[cfg(feature = "local")]
        let builder = builder.add(local::Local);

        #[cfg(feature = "remote")]
        let builder = builder.add(remote::Remote);

        #[cfg(feature = "server")]
        let builder = builder.add(server::Server);

        #[cfg(feature = "ui")]
        let builder = builder.add(ui::Ui);

        builder
    }
}
