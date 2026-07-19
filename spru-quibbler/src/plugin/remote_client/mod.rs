mod join_lobby;
pub use join_lobby::{JoinLobby, StartJoinLobby};

use bevy::prelude;

pub(crate) struct RemoteClient;

impl RemoteClient {
   
}

impl prelude::Plugin for RemoteClient {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_plugins(spru_bevy::client::remote::Plugin::<crate::Client>::default())
        ;
    }
}