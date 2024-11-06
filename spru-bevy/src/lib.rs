pub use spru::*;

pub mod item;
pub mod server;
pub use server::BevyServer;

pub struct SpruPlugin;

impl Default for SpruPlugin {
    fn default() -> Self {
        Self
    }
}

impl bevy::prelude::Plugin for SpruPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<item::lookup::EntityMap>();
    }
}

