use bevy::{ecs::system::IntoSystem as _, prelude};
use bevy_egui::egui;
use spru_bevy::server::resource::ServerMap;

use crate::plugin::ui;

pub(super) struct Plugin;

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                Self::log_ui.pipe(crate::error_to_console),
            ))
            ;
    }
}

impl Plugin {
    fn log_ui(
        mut egui: bevy_egui::EguiContexts,
        server_map: prelude::Res<ServerMap>,
        active_game: prelude::Res<ui::ActiveGame>,
        q_log: prelude::Query<(
            &crate::Log,
        )>,
    ) -> prelude::Result {
        if let Some(active_game_id) = active_game.0
            && let Some(entity) = server_map.get(active_game_id)
            && let Ok((log, )) = q_log.get(entity)
        {
            let ctx = egui.ctx_mut()?;
            let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
            let mut ui = egui::Ui::new(ctx.clone(), "server_log".into(), builder);

            egui::Panel::bottom("server_log").show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(64.)
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        egui::Grid::new("log").striped(true).show(ui, |ui| {
                            for log_line in log.iter().rev() {
                                ui.label(log_line);
                                ui.end_row();
                            }
                        });
                    });
            });
        }

        Ok(())
    }
}