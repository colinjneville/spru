use std::collections::{HashMap, HashSet};

use bevy::prelude;
use bevy_egui::egui;

#[cfg(feature = "server")]
use spru_bevy::server::resource::ServerMap;

#[cfg(feature = "client")]
use spru_bevy::client::resource::ClientMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(prelude::SystemSet)]
enum UiPhase {
    Application,
    GameSelect,
    ClientSelect,
    Server,
    Client,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(bevy::ecs::schedule::ScheduleLabel)]
pub struct UiPhaseLabel;

#[derive(Debug, Default, PartialEq, prelude::Resource)]
struct WorldInspectorToggle(bool);

#[derive(Debug)]
struct DefaultTrue(bool);

impl Default for DefaultTrue {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Default)]
#[derive(prelude::Resource)]
struct ActiveGame(Option<spru::game::Id>);

#[derive(Default)]
#[derive(prelude::Resource)]
struct ActiveClient(HashMap<spru::game::Id, spru::player::Id>);

impl ActiveClient {
    pub fn get(&self, active_game: spru::game::Id) -> Option<spru::player::Id> {
        self.0.get(&active_game).copied()
    }

    pub fn set(&mut self, active_game: spru::game::Id, value: Option<spru::player::Id>) {
        if let Some(value) = value {
            self.0.insert(active_game, value);
        } else {
            self.0.remove(&active_game);
        }
    }
}

fn render_card(ui: &mut egui::Ui, card: &crate::data::Card, is_unplayed: bool) -> egui::Response {
    let color = if is_unplayed {
        egui::Color32::from_rgb(145, 91, 87)
    } else {
        egui::Color32::WHITE
    };
    let text = format!("{}\n{}", card.face().letters_str(), card.face().points());

    let override_text_color = ui.visuals_mut().override_text_color.replace(egui::Color32::BLACK);
    let button = egui::Button::new(text)
        .min_size(egui::Vec2::new(48., 64.))
        .corner_radius(8.)
        .fill(color)
        .stroke(egui::Stroke::new(4., egui::Color32::BLACK));

    let response = ui.add(button);

    ui.visuals_mut().override_text_color = override_text_color;

    response
}

#[derive(Debug)]
struct UiData {
    active_game_id: spru::game::Id,
    active_client_id: spru::player::Id,
    active_client_entity: prelude::Entity,
    has_pending_interactions: bool,
    snapshot: UiSnapshot,
}

fn cast_option<T: Clone + Send + Sync + 'static>(dynamic: rhai::Dynamic) -> Option<T> {
    (!dynamic.is_unit()).then(|| dynamic.cast())
}

#[derive(Debug)]
struct UiSnapshot {
    round: i64,
    current_turn: Option<spru::player::Id>,
    discard_top: Option<crate::data::Card>,
    players: Vec<PlayerUiSnapshot>,
}

impl UiSnapshot {
    pub fn from_dynamic(dynamic: rhai::Dynamic) -> Self {
        let mut ui_snapshot = dynamic.cast::<rhai::Map>();
        let round = ui_snapshot.remove("round").unwrap().cast();
        let current_turn = cast_option(ui_snapshot.remove("current_turn").unwrap());
        let discard_top = cast_option(ui_snapshot.remove("discard_top").unwrap());
        let players = ui_snapshot.remove("players").unwrap().into_array().unwrap();
        let players = players.into_iter().map(|d| PlayerUiSnapshot::from_dynamic(d)).collect();

        Self {
            round,
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
        let played = cast_option(player_ui_snapshot.remove("played").unwrap());

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

pub(crate) struct Ui;

impl Ui {
    fn startup(
        mut commands: prelude::Commands,
    ) -> prelude::Result {
        commands.spawn(bevy::prelude::Camera2d);
        Ok(())
    }

    fn misc_input(
        keys: prelude::Res<prelude::ButtonInput<prelude::KeyCode>>,
        mut world_inspector_toggle: prelude::ResMut<WorldInspectorToggle>,
    ) {
        if keys.just_pressed(prelude::KeyCode::F1) {
            world_inspector_toggle.0 = !world_inspector_toggle.0;
        }
    }

    fn application_ui(
        mut egui: bevy_egui::EguiContexts,
        mut show_spru_help_window: prelude::Local<DefaultTrue>,
        mut show_quibbler_help_window: prelude::Local<DefaultTrue>,
    ) -> prelude::Result {
        let ctx = egui.ctx_mut()?;

        egui::Window::new("Spru Help")
            .open(&mut show_spru_help_window.0)
            .default_width(640.)
            .default_pos([32., 64.])
            .resizable([true, false])
            .show(ctx, |ui| {
                let mut help_text = include_str!("../../spru_help.txt");
                let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
                ui.add(multiline);
            });

        egui::Window::new("Quibbler Help")
            .open(&mut show_quibbler_help_window.0)
            .default_width(640.)
            .default_pos([756., 64.])
            .resizable([true, false])
            .show(ctx, |ui| {
                let mut help_text = include_str!("../../quibbler_help.txt");
                let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
                ui.add(multiline);
            });

        Ok(())
    }

    fn game_select_ui(
        world: &mut prelude::World,
        q_game_id: &mut prelude::QueryState<(
            &spru_bevy::common::component::GameId,
        )>,
        s_egui: &mut bevy::ecs::system::SystemState<(
            bevy_egui::EguiContexts,
            prelude::ResMut<ActiveGame>,
        )>,
        
    ) -> prelude::Result {
        let mut game_ids = HashSet::new();
        for (game_id, ) in q_game_id.query(world) {
            game_ids.insert(*game_id);
        }

        let mut game_ids: Vec<_> = game_ids.into_iter().collect();
        game_ids.sort_unstable();

        let (mut egui, mut active_game) = s_egui.get_mut(world);
        if game_ids.len() > 1 {
            let ctx = egui.ctx_mut()?;

            egui::TopBottomPanel::bottom("game_select").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for &game_id in &game_ids {
                            let button = egui::Button::selectable(
                                Some(*game_id) == active_game.0,
                                &game_id.friendly_display().to_string(),
                            );
                            if ui.add(button).clicked() {
                                active_game.0 = Some(*game_id);
                            }
                        }
                    });
                });
        } else if game_ids.len() == 1 && active_game.0 != Some(*game_ids[0]) {
            active_game.0 = Some(*game_ids[0]);
        } else if game_ids.is_empty() && active_game.0 != None {
            active_game.0 = None;
        }

        Ok(())
    }

    #[cfg(feature = "client")]
    fn client_select_ui(
        world: &mut prelude::World,
        s_egui: &mut bevy::ecs::system::SystemState<(
            bevy_egui::EguiContexts,
            prelude::ResMut<ActiveClient>,
        )>,
    ) -> prelude::Result {
        if let Some(active_game) = world.resource::<ActiveGame>().0 {
            let client_map = world.resource::<ClientMap>();
            let mut player_names = vec![];
            for (game_id, client_id, entity) in client_map.iter() {
                if active_game == game_id {
                    let username: String = spru_bevy::client::eval::<crate::Client, _, _>(world, entity, &crate::Language::default(), r#"
                        context.root.players.get(args).data.username
                    "#, client_id)?;

                    player_names.push((client_id, username));
                }
            }
            player_names.sort_unstable_by_key(|(id, _)| *id);

            let (mut egui, mut active_client) = s_egui.get_mut(world);
            let ctx = egui.ctx_mut()?;
            
            if player_names.len() > 1 {
                egui::TopBottomPanel::bottom("player_select").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for (client_id, username) in &player_names {
                            let button = egui::Button::selectable(
                                Some(*client_id) == active_client.get(active_game),
                                username,
                            );
                            if ui.add(button).clicked() {
                                active_client.set(active_game, Some(*client_id));
                            }
                        }
                    });
                });
            } else if player_names.len() == 1 {
                active_client.set(active_game, Some(player_names[0].0));
            }
        }

        Ok(())
    }

    #[cfg(feature = "server")]
    fn server_ui_control(
        mut egui: bevy_egui::EguiContexts,
        server_map: prelude::Res<ServerMap>,
        mut add_player_string: prelude::Local<String>,
        active_game: prelude::Res<ActiveGame>,
        mut q_server: prelude::Query<(
            &mut spru_bevy::server::component::FromUser<crate::Server>,
        )>,
    ) -> prelude::Result {
        let Some(active_game_id) = active_game.0 else {
            return Ok(());
        };
        let Some(server_entity) = server_map.get(active_game_id) else {
            return Ok(());
        };

        let ctx = egui.ctx_mut()?;
        
        let (mut server_from_user, ) = q_server.get_mut(server_entity)?;

        egui::TopBottomPanel::top("server_control").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let response = ui
                        .button("Add player")
                        .on_hover_text("Input player display name");

                    let confirmed = ui
                        .text_edit_singleline(&mut *add_player_string)
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if (response.clicked() || confirmed) && !add_player_string.is_empty() {
                        let player_init_in =
                            crate::player::Input::new(std::mem::take(&mut *add_player_string));
                        server_from_user.add_player(player_init_in);
                    }
                });

                if ui.button("Start game").clicked() {
                    server_from_user.manual_trigger(crate::reaction::Trigger::StartGame);
                }
            });
        });

        Ok(())
    }

    #[cfg(feature = "client")]
    fn client_ui(
        world: &mut prelude::World,
    ) -> prelude::Result {
        let ui_data: Option<UiData> = world.run_system_cached(Self::client_ui_extract)?;
        let _: () = world.run_system_cached(Self::client_ui_log)?;
        let _: () = world.run_system_cached_with(Self::client_ui_player_view, ui_data)?;
        Ok(())
    }

    #[cfg(feature = "client")]
    fn client_ui_extract(
        world: &prelude::World,
        client_map: prelude::Res<ClientMap>,
        active_game: prelude::Res<ActiveGame>,
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

    #[cfg(feature = "server")]
    fn server_ui_log(
        mut egui: bevy_egui::EguiContexts,
        server_map: prelude::Res<ServerMap>,
        active_game: prelude::Res<ActiveGame>,
        q_log: prelude::Query<(
            &crate::Log,
        )>,
    ) -> prelude::Result {
        if let Some(active_game_id) = active_game.0
            && let Some(entity) = server_map.get(active_game_id)
            && let Ok((log, )) = q_log.get(entity)
        {
            let ctx = egui.ctx_mut()?;

            egui::TopBottomPanel::bottom("server_log").show(ctx, |ui| {
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

    #[cfg(feature = "client")]
    fn client_ui_log(
        mut egui: bevy_egui::EguiContexts,
        client_map: prelude::Res<ClientMap>,
        active_game: prelude::Res<ActiveGame>,
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
            let ctx = egui.ctx_mut()?;

            egui::TopBottomPanel::bottom("client_log").show(ctx, |ui| {
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

    #[cfg(feature = "client")]
    fn client_ui_player_view(
        ui_data: prelude::In<Option<UiData>>,
        mut egui: bevy_egui::EguiContexts,
        mut play_string: prelude::Local<String>,
        mut q_client_from_user: prelude::Query<(
            &mut spru_bevy::client::component::FromUser<crate::Client>,
        )>
    ) -> prelude::Result {
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
        let (mut from_user, ) = q_client_from_user.get_mut(active_client_entity)
            .map_err(|_| "Client not found")?;

        egui::TopBottomPanel::bottom("player_view").show(ctx, |ui| {
            ui.vertical(|ui| -> prelude::Result {
                ui.heading(format!("{}'s view", active_client_snapshot.name));
                ui.separator();

                ui.label(format!("Round {} of 8", snapshot.round + 1));

                if let Some(player_turn_snapshot) = player_turn_snapshot {
                    ui.label(format!("{}'s turn ({})", player_turn_snapshot.name, player_turn_snapshot.fsm_state));
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Draw from deck").clicked() {
                        from_user.stage_interaction(crate::interaction::draw::new(true).into());
                    }
                    if let Some(discard_top) = &snapshot.discard_top {
                        let button_message = format!("Draw '{}' from discard ({} points)", discard_top.face().letters_str(), discard_top.face().points);
                        if ui.button(button_message).clicked() {
                            from_user.stage_interaction(crate::interaction::draw::new(false).into());
                        }
                    }
                });

                ui.label("Hand (click to discard)");

                ui.horizontal(|ui| {
                    for card in &active_client_snapshot.hand {
                        if render_card(ui, card, false).clicked() {
                            from_user.stage_interaction(crate::interaction::discard::new(card.clone()).into());
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
                        
                        from_user.stage_interaction(crate::interaction::play::new(Some(play)).into());
                    }

                    if ui.button("Pass").clicked() {
                        from_user.stage_interaction(crate::interaction::play::new(None).into());
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
                                                render_card(ui, card, false);
                                            }
                                            ui.add_space(24.);
                                        }
                                    }   

                                    if !played.is_full() {
                                        for card in played.unused() {
                                            render_card(ui, card, true);
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
                        from_user.apply_all_interactions();
                    }
                    if ui.add(revert_button).clicked() {
                        from_user.revert_all_interactions();
                    }
                });

                Ok(())
            }).inner
        }).inner?;

        Ok(())
    }
}

impl prelude::Plugin for Ui {
    fn build(&self, app: &mut bevy::app::App) {
        use prelude::IntoScheduleConfigs as _;

        app
            .add_plugins((
                bevy_egui::EguiPlugin::default(),
                bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                    .run_if(bevy::ecs::schedule::common_conditions::resource_equals(WorldInspectorToggle(true))),
            ))
            .init_resource::<WorldInspectorToggle>()
            .init_resource::<ActiveGame>()
            .init_resource::<ActiveClient>()
            .edit_schedule(bevy_egui::EguiPrimaryContextPass, |schedule| {
                schedule.configure_sets((
                    UiPhase::Application,
                    UiPhase::GameSelect,
                    UiPhase::ClientSelect,
                    UiPhase::Server,
                    UiPhase::Client,
                ).chain());
            })
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                Self::application_ui.in_set(UiPhase::Application),
                Self::game_select_ui.in_set(UiPhase::GameSelect),
                #[cfg(feature = "client")]
                Self::client_select_ui.in_set(UiPhase::ClientSelect),
                #[cfg(feature = "server")]
                (Self::server_ui_control, Self::server_ui_log).in_set(UiPhase::Server),
                #[cfg(feature = "client")]
                Self::client_ui.in_set(UiPhase::Client),
            ))
            .add_systems(prelude::Startup, Self::startup)
            .add_systems(prelude::Update, Self::misc_input)
        ;

        // panel_ui.pipe(error_to_console),
    }
}