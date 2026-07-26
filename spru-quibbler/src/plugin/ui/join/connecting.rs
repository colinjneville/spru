use bevy::prelude;
use bevy_egui::egui;

use crate::plugin;

pub(super) fn ui(
    mut commands: prelude::Commands,
    mut egui: bevy_egui::EguiContexts,
    timer: prelude::Res<prelude::Time>,
    q_connecting: prelude::Query<(
        prelude::Entity,
        &plugin::join::Connecting,
    )>,
) -> prelude::Result {
    for (entity, _connecting, ) in q_connecting {
        let ctx = egui.ctx_mut()?;
        
        let dots = (timer
        .elapsed()
        .subsec_millis() / 250
        % 4) as usize;

        let window_center = ctx.content_rect().center();
        let window_size = egui::Vec2::new(240., 160.);
        let window_rect = egui::Rect::from_center_size(window_center, window_size);

        egui::Window::new("join_lobby_connecting")
            .title_bar(false)
            .fixed_rect(window_rect)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    let mut s = "Connecting".to_string();
                    for _ in 0..dots {
                        s.push('.');
                    }

                    let text_size = egui::WidgetText::from("Connecting...")
                        .into_galley(ui, None, ui.available_width(), egui::TextStyle::Body)
                        .size();

                    ui.allocate_ui_with_layout(text_size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(s);
                    });

                    if ui.button("Cancel").clicked() {
                        use bevy::state::commands::CommandsStatesExt as _;

                        commands.entity(entity)
                            .despawn();
                        commands.set_state(crate::AppState::MainMenu);
                    }
                });
            })
            ;
    }

    Ok(())
}
