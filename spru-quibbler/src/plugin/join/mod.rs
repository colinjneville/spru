mod config;
pub use config::Config;
mod connecting;
pub use connecting::Connecting;
mod join_lobby;
pub use join_lobby::{JoinLobby, StartJoinLobby};

use bevy::prelude;

pub(crate) struct Join;

impl Join {
   
}

impl prelude::Plugin for Join {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_plugins(spru_bevy::client::remote::Plugin::<crate::Client>::default())
            .add_observer(Connecting::on_connected)
            ;
    }
}

pub(super) fn start(

) {
    config_state.set(join::ConfigJoin::default());
    next_state.set_if_neq(crate::AppState::Config);
}