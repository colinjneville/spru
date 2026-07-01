use bevy::prelude;
use bevy_egui::egui;

#[derive(Debug, Default)]
pub(super) struct ConfigDedicatedServer {
    step: usize,
    // player_panel: ConfigManyPlayerPanel,
}

impl super::Config for ConfigDedicatedServer {
    fn show(&mut self, ctx: &mut egui::Context) -> bool {
        self.step == 1
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        todo!()
    }
}