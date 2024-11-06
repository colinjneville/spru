pub mod action;
pub use action::Catalog;
pub mod component;
pub mod data;
pub mod game;
pub mod interaction;
use spru_bevy::item::{self, lookup};
use interaction::Interaction;
mod player;

use amass::amass_telety;

use bevy::prelude::*;

fn main() {
    bevy::app::App::new()
        .add_plugins(bevy::DefaultPlugins)
        .add_plugins(spru_bevy::SpruPlugin)
        .add_systems(Startup, spru_startup)
        .add_systems(Startup, startup)
        .run();
}

type Server = spru_bevy::BevyServer<item::IdT<game::Root>, player::Init>;

fn spru_startup(
    world: &mut World,
) {

    let mut lookup = item::BevyLookupMut::new(world);
    let mut server = Server::new(&mut lookup, game::Init, game::Input, player::Init)
        .unwrap();

    server.add_player(&mut lookup, player::Input::new("player1".to_string())).unwrap();

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
    commands.spawn(Camera2dBundle::default());
}