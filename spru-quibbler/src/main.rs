mod actions;
use std::fmt;

pub use actions::Actions;
pub mod data;
pub mod game;
pub mod hand;
pub mod interaction;
pub use interaction::Interaction;
pub mod round;
use spru::item::IdT;
mod play;
pub use play::Play;
mod player;
mod reaction;
pub use reaction::Reaction;
pub mod trigger;
pub use trigger::Trigger;
mod state;
pub use state::State;

use bevy::prelude::*;

type Client = spru::client::Impl<State, Actions, IdT<game::Root>, Interaction, game::Outcome>;
type Server = spru::server::Impl<State, Actions, IdT<game::Root>, Interaction, Reaction, player::Init>;

fn main() {
    bevy::app::App::new()
        .add_plugins(bevy::DefaultPlugins)
        .add_plugins(spru_bevy::client::Plugin::<Client>::default())
        .add_plugins(spru_bevy::server::Plugin::<Server>::default())
        .add_plugins(spru_bevy::local::Plugin::<Server, Client>::default())
        .add_systems(Startup, spru_startup)
        .add_systems(Startup, startup)
        .run();
}

#[derive(Debug)]
pub struct Error(pub anyhow::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

macro_rules! anyhow {
    ($msg:literal $(,)?) => {
        $crate::Error(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::Error(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error(anyhow::anyhow!($fmt, $($arg)*))
    }
}
pub(crate) use anyhow;

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::anyhow!($msg).into());
    };
    ($err:expr $(,)?) => {
        return Err($crate::::anyhow!($err).into());
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::::anyhow!($fmt, $($arg)*).into());
    }
}
pub(crate) use bail;


fn spru_startup(
    world: &mut World,
) {
    let (_e_server, game_id) = spru_bevy::server::command::SpawnServer::<Server, _> {
        game_init: game::Init,
        player_init: player::Init,
        reaction: Reaction,
    }.apply(world)
        .expect("SpawnServer failed");

    for i in 0..4 {
        let _player_id = spru_bevy::server::command::AddPlayer::<Server> {
            game_id,
            player_init_input: player::Input {
                username: format!("Player {i}"),
            },
        }.apply(world)
            .expect("AddPlayer failed");
    }
}

fn startup(
    mut commands: Commands,
) {
    commands.spawn(bevy::prelude::Camera2d::default());
}