use std::time;

use bevy::prelude;

pub(crate) struct DedicatedClient;

impl DedicatedClient {
   
}

impl prelude::Plugin for DedicatedClient {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins(aeronet_webtransport::client::WebTransportClientPlugin)
        ;
    }
}