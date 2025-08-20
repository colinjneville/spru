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
use spru_bevy::item;
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

fn main() {
    bevy::app::App::new()
        .add_plugins(bevy::DefaultPlugins)
        .add_plugins(spru_bevy::SpruPlugin)
        .add_systems(Startup, spru_startup)
        .add_systems(Startup, startup)
        .run();
}

type Server = spru_bevy::Server<State, Actions, item::IdT<game::Root>, player::Init, Interaction, Reaction>;

#[derive(Debug)]
// #[derive(thiserror::Error)]
// #[error("{0}")]
pub struct Error(anyhow::Error);

impl<T: Into<anyhow::Error>> From<T> for Error {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return anyhow::anyhow!($msg).into();
    };
    ($err:expr $(,)?) => {
        return anyhow::anyhow!($err).into();
    };
    ($fmt:expr, $($arg:tt)*) => {
        return anyhow::anyhow!($fmt, $($arg)*).into();
    }
}
pub(crate) use bail;


fn spru_startup(
    world: &mut World,
) {

    let mut lookup = item::BevyLookupMut::new(world);
    let mut server = Server::new(game::Init, player::Init, Reaction)
        .unwrap();

    let spru::server::Output {
        outbound,
        events,
        ret,
    } = server.add_player(spru::server::add_player::Arg {
        init_input: player::Input::new("player1".to_string()),
    }).unwrap();

    let spru::server::add_player::Ret {
        client_init,
        player_id,
    } = ret;

    world.insert_resource(server);

    let lookup = item::BevyLookup::new(world);

    for player in world.resource::<Server>().players() {
        use spru_bevy::item::lookup::OfType;
        let player_root = lookup.lookup(player.root()).unwrap();
        println!("{}", player_root.data.username);
    }
}

fn startup(
    mut commands: Commands,
) {
    commands.spawn(bevy::prelude::Camera2d::default());
}