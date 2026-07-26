use bevy::prelude;
use bevy_egui::egui;

pub(super) struct Plugin;

impl Plugin {
    
}

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        // todo!()
    }
}

#[derive(Debug, Default)]
pub(super) struct ConfigDedicatedServer {
    step: usize,
    // player_panel: ConfigManyPlayerPanel,
}

impl super::Config for ConfigDedicatedServer {
    fn title(&self) -> &'static str {
        "Run Dedicated Server"
    }

    fn panels(&mut self) -> (Vec<&mut dyn super::ConfigPanel>, &mut usize) {
        (
            vec![
                // &mut self.player_panel as &mut dyn super::ConfigPanel,
            ],
            &mut self.step,
        )
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        todo!()
    }
}