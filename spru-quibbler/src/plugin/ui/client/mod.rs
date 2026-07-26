mod active_client;
pub(crate) use active_client::ActiveClient;
mod game_outcome;

use bevy::{ecs::system::IntoSystem as _, prelude};

use spru_bevy::client::resource::ClientMap;

use crate::plugin::ui;

pub(super) struct Plugin;

impl Plugin {
    // fn client_ui(
    //     world: &mut prelude::World,
    // ) -> prelude::Result {
    //     let ui_data: Option<UiData> = world.run_system_cached(Self::client_ui_extract)?;
    //     let _: () = world.run_system_cached(Self::client_ui_log)?;
    //     let _: () = world.run_system_cached_with(Self::client_ui_player_view, ui_data)?;
    //     Ok(())
    // }

    fn ui_extract(
        world: &prelude::World,
        client_map: prelude::Res<ClientMap>,
        active_game: prelude::Res<ui::ActiveGame>,
        active_client: prelude::Res<ActiveClient>,
        q_client: prelude::Query<(
            &spru_bevy::client::component::Runner<crate::Client>,
        )>,
    ) -> prelude::Result<Option<UiData>> {
        if let Some(active_game_id) = active_game.0 
            && let Some(active_client_id) = active_client.get(active_game_id)
            && let Some(active_client_entity) = client_map.get(active_game_id, active_client_id)
        {
            let ui_snapshot: rhai::Dynamic = spru_bevy::client::eval::<crate::Client, _, _>(world, active_client_entity, &crate::Language::default(), r#"
                let players = context.root.players.ids.zip(context.root.players.players, |id, player| {
                    #{
                        id: id,
                        name: player.data.username,
                        score: player.score.value,
                        hand: player.hand.items,
                        fsm_state: player.fsm.current,
                        played: player.played.value,
                    }
                });
                #{
                    round: context.root.round.value,
                    total_rounds: context.root.settings.last_hand + 1 - context.root.settings.first_hand,
                    current_turn: context.root.current_turn.current,
                    discard_top: context.root.discard.top,
                    players: players,
                }
            "#, ())?;

            let snapshot = UiSnapshot::from_dynamic(ui_snapshot);

            let (client, ) = q_client.get(active_client_entity)?;
            let has_pending_interactions = client.pending_interactions().next().is_some();

            Ok(Some(UiData {
                active_game_id,
                active_client_id,
                active_client_entity,
                has_pending_interactions,
                snapshot,
            }))
        } else {
            Ok(None)
        }
    }

    fn ui_player_view(
        ui_data: prelude::In<Option<UiData>>,
        mut commands: prelude::Commands,
        mut egui: bevy_egui::EguiContexts,
        mut play_string: prelude::Local<String>,
    ) -> prelude::Result {
        use bevy_egui::egui;

        let prelude::In(Some(ui_data)) = ui_data else {
            return Ok(());
        };

        let UiData {
            active_game_id: _active_game_id,
            active_client_id,
            active_client_entity,
            has_pending_interactions,
            snapshot,
        } = ui_data;

        let Some(active_client_snapshot) = snapshot.players.iter().filter(|p| p.id == active_client_id).next() else {
            return Ok(());
        };
        let player_turn_snapshot = snapshot.players.iter().filter(|p| Some(p.id) == snapshot.current_turn).next();

        let ctx = egui.ctx_mut()?;
        let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
        let mut ui = egui::Ui::new(ctx.clone(), "player_view".into(), builder);
        
        egui::Panel::bottom("player_view").show(ctx, |ui| {
            ui.vertical(|ui| -> prelude::Result {
                ui.heading(format!("{}'s view", active_client_snapshot.name));
                ui.separator();

                ui.label(format!("Round {} of {}", snapshot.round + 1, snapshot.total_rounds));

                if let Some(player_turn_snapshot) = player_turn_snapshot {
                    ui.label(format!("{}'s turn ({})", player_turn_snapshot.name, player_turn_snapshot.fsm_state));
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Draw from deck").clicked() {
                        commands.entity(active_client_entity)
                                .queue(spru_bevy::client::command::StageInteraction::<crate::Client>::new(crate::interaction::draw::new(true).into()));
                    }
                    if let Some(discard_top) = &snapshot.discard_top {
                        let button_message = format!("Draw '{}' from discard ({} points)", discard_top.face().letters_str(), discard_top.face().points);
                        if ui.button(button_message).clicked() {
                            commands.entity(active_client_entity)
                                .queue(spru_bevy::client::command::StageInteraction::<crate::Client>::new(crate::interaction::draw::new(false).into()));
                        }
                    }
                });

                ui.label("Hand (click to discard)");

                ui.horizontal(|ui| {
                    for card in &active_client_snapshot.hand {
                        if ui::render_card(ui, card, false).clicked() {
                            commands.entity(active_client_entity)
                                .queue(spru_bevy::client::command::StageInteraction::<crate::Client>::new(crate::interaction::discard::new(card.clone()).into()));
                        }
                    }
                });

                let confirmed = ui.text_edit_singleline(&mut *play_string).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                ui.horizontal(|ui| -> prelude::Result {
                    if ui.button("Play word(s)")
                        .on_hover_text("Separate each played word with a space, do not type unused letters")
                        .clicked() || confirmed
                    {
                        let mut words = std::mem::take(&mut *play_string)
                            .into_bytes();
                        words.make_ascii_uppercase();

                        let play = crate::Play::parsed(&active_client_snapshot.hand, &words)
                            .map_err(|c| format!("Can't play '{}', missing '{c}'", String::from_utf8(words).unwrap()))?;
                        
                        commands.entity(active_client_entity)
                                .queue(spru_bevy::client::command::StageInteraction::<crate::Client>::new(crate::interaction::play::new(play).into()));
                    }

                    if ui.button("Pass").clicked() {
                        commands.entity(active_client_entity)
                                .queue(spru_bevy::client::command::StageInteraction::<crate::Client>::new(crate::interaction::play::pass().into()));
                    }

                    Ok(())
                }).inner?;

                ui.separator();

                ui.horizontal(|ui| -> prelude::Result {
                    for player in &snapshot.players {
                        let PlayerUiSnapshot { 
                            name, 
                            score, 
                            played, 
                            ..
                        } = player;
                        
                        ui.vertical(|ui| {
                            ui.label(format!("{name}: {score} points"));
                            ui.horizontal(|ui| {
                                if let Some(played) = &played {
                                    if played.word_count() > 0 {
                                        for word in played.words() {
                                            for card in word {
                                                ui::render_card(ui, card, false);
                                            }
                                            ui.add_space(24.);
                                        }
                                    }   

                                    if !played.is_full() {
                                        for card in played.unused() {
                                            ui::render_card(ui, card, true);
                                        }
                                    }
                                }
                            });
                            if let Some(played) = &played {
                                let play_score = played.base_score();
                                let word_count = played.word_count();
                                let max_word_len = played.max_word_len();
                                ui.label(format!("{play_score} points"));
                                ui.label(format!("Most words: {word_count} words"));
                                ui.label(format!("Longest word: {max_word_len} letters"));
                            }
                        });
                    }
                    Ok(())
                }).inner?;

                ui.separator();
                ui.label("Local Changes:");
                ui.horizontal(|ui| {
                    let mut apply_button = egui::Button::new("Apply");
                    let mut revert_button = egui::Button::new("Revert");
                    if has_pending_interactions {
                        apply_button = apply_button.stroke(egui::Stroke::new(1.5, egui::Color32::ORANGE));
                        revert_button = revert_button.stroke(egui::Stroke::new(1.5, egui::Color32::ORANGE));
                    }
                    if ui.add(apply_button).clicked() {
                        commands.entity(active_client_entity)
                            .queue(spru_bevy::client::command::ApplyInteractions::<crate::Client>::all());
                    }
                    if ui.add(revert_button).clicked() {
                        commands.entity(active_client_entity)
                            .queue(spru_bevy::client::command::RevertInteractions::<crate::Client>::all());
                    }
                });

                Ok(())
            }).inner
        }).inner?;

        Ok(())
    }

    fn client_ui_log(
        mut egui: bevy_egui::EguiContexts,
        client_map: prelude::Res<ClientMap>,
        active_game: prelude::Res<ui::ActiveGame>,
        active_client: prelude::Res<ActiveClient>,
        q_log: prelude::Query<(
            &crate::Log,
        )>,
    ) -> prelude::Result {
        if let Some(active_game_id) = active_game.0
            && let Some(active_client_id) = active_client.get(active_game_id) 
            && let Some(entity) = client_map.get(active_game_id, active_client_id)
            && let Ok((log, )) = q_log.get(entity)
        {
            use bevy_egui::egui;

            let ctx = egui.ctx_mut()?;
            let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
            let mut ui = egui::Ui::new(ctx.clone(), "client_log".into(), builder);

            egui::Panel::bottom("client_log").show(ctx, |ui| {
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

    fn on_add_runner(
        add_runner: prelude::On<prelude::Add, spru_bevy::client::component::Runner<crate::Client>>,
        mut active_client: prelude::ResMut<ActiveClient>,
        q_runner: prelude::Query<(
            &spru_bevy::common::component::GameId,
            &spru_bevy::client::component::ClientId,
        )>,
    ) {
        if let Ok((game_id, client_id)) = q_runner.get(add_runner.entity) {
            if active_client.get(**game_id).is_none() {
                active_client.set(**game_id, Some(**client_id));
            }
        }
    }
}

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .init_resource::<ActiveClient>()
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                // Self::client_select_ui.pipe(crate::error_to_console).in_set(UiPhase::ClientSelect),
                Self::ui_extract.pipe(
                    Self::ui_player_view.pipe(
                        crate::error_to_console
                    )
                ),
                game_outcome::GameOutcome::ui.pipe(crate::error_to_console),
            ))
            ;
    }
}

#[derive(Debug)]
struct UiData {
    active_game_id: spru::game::Id,
    active_client_id: spru::player::Id,
    active_client_entity: prelude::Entity,
    has_pending_interactions: bool,
    snapshot: UiSnapshot,
}

#[derive(Debug)]
struct UiSnapshot {
    round: i64,
    total_rounds: i64,
    current_turn: Option<spru::player::Id>,
    discard_top: Option<crate::data::Card>,
    players: Vec<PlayerUiSnapshot>,
}

impl UiSnapshot {
    pub fn from_dynamic(dynamic: rhai::Dynamic) -> Self {
        let mut ui_snapshot = dynamic.cast::<rhai::Map>();
        let round = ui_snapshot.remove("round").unwrap().cast();
        let total_rounds = ui_snapshot.remove("total_rounds").unwrap().cast();
        let current_turn = super::cast_option(ui_snapshot.remove("current_turn").unwrap());
        let discard_top = super::cast_option(ui_snapshot.remove("discard_top").unwrap());
        let players = ui_snapshot.remove("players").unwrap().into_array().unwrap();
        let players = players.into_iter().map(|d| PlayerUiSnapshot::from_dynamic(d)).collect();

        Self {
            round,
            total_rounds,
            current_turn,
            discard_top,
            players,
        }
    }
}

#[derive(Debug)]
struct PlayerUiSnapshot {
    id: spru::player::Id,
    name: String,
    score: i64,
    hand: Vec<crate::data::Card>,
    fsm_state: crate::player::machine::State,
    played: Option<crate::Play>,
}

impl PlayerUiSnapshot {
    pub fn from_dynamic(dynamic: rhai::Dynamic) -> Self {
        let mut player_ui_snapshot = dynamic.cast::<rhai::Map>();
        let id = player_ui_snapshot.remove("id").unwrap().cast();
        let name = player_ui_snapshot.remove("name").unwrap().into_string().unwrap();
        let score = player_ui_snapshot.remove("score").unwrap().cast();
        let hand = player_ui_snapshot.remove("hand").unwrap().into_typed_array().unwrap();
        let fsm_state = player_ui_snapshot.remove("fsm_state").unwrap().cast();
        let played = super::cast_option(player_ui_snapshot.remove("played").unwrap());

        Self {
            id,
            name,
            score,
            hand,
            fsm_state,
            played,
        }
    }
}
