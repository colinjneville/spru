use std::cmp;

use bevy::prelude;
use bevy_egui::egui;

use crate::game;

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct GameOutcome {
    game_outcome: game::Outcome,
}

impl GameOutcome {
    pub fn new(game_outcome: game::Outcome) -> Self {
        Self {
            game_outcome,
        }
    }
}

impl GameOutcome {
    pub(super) fn ui(
        mut commands: prelude::Commands,
        mut egui: bevy_egui::EguiContexts,
        q_game_outcome: prelude::Single<(
            prelude::Entity,
            &GameOutcome,
        )>,
    ) -> prelude::Result {
        let ctx = egui.ctx_mut()?;

        let (entity, game_outcome, ) = *q_game_outcome;
        let GameOutcome {
            game_outcome,
        } = game_outcome;

        let window_center = ctx.content_rect().center();

        egui::Window::new("Game Over")
            .fixed_pos(window_center)
            .pivot(egui::Align2::CENTER_CENTER)
            .resizable(false)
            .collapsible(false)
            .auto_sized()
            .show(ctx, |ui| {
                egui::Grid::new("Game Over Grid")
                    .striped(true)
                    .num_columns(3)
                    .show(ui, |ui| {
                        ui.advance_cursor_after_rect(egui::Rect::ZERO);
                        ui.horizontal_centered(|ui| {
                            ui.label("Player");
                        });
                        ui.horizontal_centered(|ui| {
                            ui.label("Score");
                        });
                        
                        ui.end_row();

                        let mut final_scores = game_outcome.final_scores.clone();
                        final_scores.sort_by_key(|(_, _, score)| cmp::Reverse(*score));

                        for (player_id, name, score) in final_scores {
                            if game_outcome.winners.contains(&player_id) {
                                // A filled star
                                ui.label("\u{2605}");
                            } else {
                                ui.advance_cursor_after_rect(egui::Rect::ZERO);
                            }

                            ui.label(name);
                            ui.label(score.to_string());

                            ui.end_row();
                        }

                        
                    });
                ui.horizontal_centered(|ui| {
                    if ui.button("Close").clicked() {
                        use bevy::state::commands::CommandsStatesExt as _;

                        commands.entity(entity)
                            .queue(spru_bevy::client::command::Shutdown::<crate::Client>::new(true))
                            ;

                        commands.set_state(crate::AppState::MainMenu);
                        
                    }
                });
            });
            

        Ok(())
    }
}