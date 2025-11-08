#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod actions;
use std::{collections::HashMap, fmt};

pub use actions::Actions;
pub mod data;
pub mod game;
pub mod interaction;
pub use interaction::Interaction;
mod play;
pub mod round;
pub use play::Play;
mod player;
mod reaction;
pub use reaction::Reaction;
mod state;
use spru::{common::error::PsuedoError as _, item::Storage as _};
use spru_bevy::{client::ClientSSS as _, server::ServerSSS as _};
use spru_util::player_map;
pub use state::State;

use bevy::{ecs::system::IntoSystem, prelude};
use spru_bevy::client::component::Item;

type Client = spru::client::Impl<Interaction, game::Outcome>;
type Server = spru::server::Impl<Interaction, Reaction, player::Init>;
type Common = <Client as spru::Client>::Common;

fn main() {
    use prelude::PluginGroup as _;

    let _frame_duration = std::time::Duration::from_secs_f32(1. / 30.);

    #[rustfmt::skip]
    bevy::app::App::new()
        .add_plugins((
            bevy::DefaultPlugins.set(bevy::log::LogPlugin {
                filter: "spru=trace,spru_bevy=info,spru_quibbler=trace".to_string(),
                ..Default::default()
            }),
            spru_bevy::client::Plugin::<Client>::default(),
            spru_bevy::server::Plugin::<Server>::default(),
            spru_bevy::local::Plugin::<Server, Client>::default(),
            bevy_egui::EguiPlugin::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                .run_if(bevy::ecs::schedule::common_conditions::resource_equals(WorldInspectorToggle(true))),
        ))
        .init_resource::<GameId>()
        .init_resource::<ClientIds>()
        .init_resource::<ActiveClientId>()
        .init_resource::<WorldInspectorToggle>()
        .init_resource::<Log>()
        .add_systems(prelude::Startup, 
            (
                startup,
            )
        )
        .add_systems(prelude::FixedUpdate,
            (
                print_piles,
            ),
        )
        .add_systems(prelude::Update,
            (
                misc_input,
            )
        )
        .add_systems(bevy_egui::EguiPrimaryContextPass,
            (
                panel_ui.pipe(error_to_console),
            )
        )
        // Server Init
        .add_observer(
            |server_init: prelude::On<spru_bevy::server::event::Init<Server>>,
             mut game_id: prelude::ResMut<GameId>,
             mut q_server: prelude::Query<(
                &spru_bevy::common::component::GameId,
                &mut spru_bevy::server::component::FromUser<Server>,
            )>|
             -> prelude::Result {
                let gid = *server_init.result.as_ref().map_err(ToString::to_string)?;
                game_id.set(gid);

                let (_, mut from_user) =
                    Server::filter_mut(&mut q_server, gid).ok_or("Server not found")?;

                for username in ["Alice", "Bob"] {
                    from_user.add_player(player::Input {
                        username: username.to_string(),
                    });
                }

                Ok(())
            },
        )
        // Client Init
        .add_observer(
            |client_init: prelude::On<spru_bevy::client::event::Init<Client>>,
             mut client_ids: prelude::ResMut<ClientIds>|
             -> prelude::Result {
                let client_id = *client_init.result.as_ref().map_err(ToString::to_string)?;
                client_ids.0.push(client_id);
                Ok(())
            },
        )
        // User-facing log
        .add_observer(
            |add_player: prelude::On<spru_bevy::server::event::AddPlayer<Server>>,
             mut log: prelude::ResMut<Log>|
            {
                let message = match &add_player.result {
                    Ok(player_id) => format!("Player {player_id} added"),
                    Err(err) => format!("Add player failed: {err}"),
                };
                log.server_log(message);
            },
        )
        .add_observer(
            |manual_trigger: prelude::On<spru_bevy::server::event::ManualTrigger<Server>>,
             mut log: prelude::ResMut<Log>|
            {
                let message = match &manual_trigger.result {
                    Ok(()) => "Manual trigger successful".to_string(),
                    Err(err) => format!("Manual trigger failed: {err}"),
                };
                log.server_log(message);
            },
        )
        .add_observer(
            |game_complete: prelude::On<spru_bevy::server::event::GameComplete<Server>>,
             mut log: prelude::ResMut<Log>|
            {
                log.server_log("Game complete");
                for (id, score) in &game_complete.game_outcome.final_scores {
                    log.server_log(format!("{id}: {score}"));
                }
            },
        )
        .add_observer(
            |stage_interaction: prelude::On<spru_bevy::client::event::StageInteraction<Client>>,
             mut log: prelude::ResMut<Log>|
            {
                let message = match &stage_interaction.result {
                    Ok(pending_id) => format!("Interaction staged ({pending_id})"),
                    Err(err) => format!("Stage failed: {err}"),
                };
                log.client_log(stage_interaction.client_id, message);
            },
        )
        .add_observer(
            |apply_interactions: prelude::On<spru_bevy::client::event::ApplyInteractions<Client>>,
             mut log: prelude::ResMut<Log>|
            {
                let message = match &apply_interactions.result {
                    Ok(count) => {
                        if *count == 0 {
                            return;
                        }
                        format!("{count} Interactions applied")
                    }
                    Err(err) => format!("Apply failed: {err}"),
                };
                log.client_log(apply_interactions.client_id, message);
            },
        )
        .add_observer(
            |revert_interactions: prelude::On<spru_bevy::client::event::RevertInteractions<Client>>,
             mut log: prelude::ResMut<Log>|
            {
                let message = match &revert_interactions.result {
                    Ok(count) => {
                        if *count == 0 {
                            return;
                        }
                        format!("{count} Interactions reverted")
                    }
                    Err(err) => format!("Revert failed: {err}"),
                };
                log.client_log(revert_interactions.client_id, message);
            },
        )
        .run();
}

#[derive(Debug, Default, prelude::Resource)]
struct Log {
    server_log: Vec<String>,
    client_logs: HashMap<spru_bevy::client::component::ClientId, Vec<String>>,
}

impl Log {
    fn server_log<S: ToString>(&mut self, message: S) {
        self.server_log.push(message.to_string());
    }

    fn client_log<S: ToString>(
        &mut self,
        client_id: spru_bevy::client::component::ClientId,
        message: S,
    ) {
        self.client_logs
            .entry(client_id)
            .or_default()
            .push(message.to_string());
    }

    fn iter_logs(
        &self,
        client_id: Option<spru_bevy::client::component::ClientId>,
    ) -> impl DoubleEndedIterator<Item = &str> {
        let client_log = if let Some(client_id) = client_id {
            self.client_logs.get(&client_id)
        } else {
            None
        }
        .map(std::ops::Deref::deref)
        // <[_]> Who knew this worked?
        .map(<[_]>::iter)
        .into_iter()
        .flatten();

        self.server_log.iter().chain(client_log).map(String::as_str)
    }
}

#[derive(Debug)]
pub struct Error(pub anyhow::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

macro_rules! anyhow {
    ($msg:literal $(,)?) => {
        $crate::Error(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::Error(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error(anyhow::anyhow!($fmt, $($arg)*))
    }
}
pub(crate) use anyhow;

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::anyhow!($msg).into());
    };
    ($err:expr $(,)?) => {
        return Err($crate::::anyhow!($err).into());
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::::anyhow!($fmt, $($arg)*).into());
    }
}
pub(crate) use bail;

#[derive(Debug, Default, PartialEq, prelude::Resource)]
struct WorldInspectorToggle(bool);

#[derive(Debug, Default, prelude::Resource)]
struct GameId(Option<spru_bevy::common::component::GameId>);

impl GameId {
    pub fn get(&self) -> spru_bevy::common::component::GameId {
        self.0.unwrap()
    }

    fn set(&mut self, game_id: spru_bevy::common::component::GameId) {
        self.0 = Some(game_id);
    }
}

#[derive(Debug, Default, prelude::Resource)]
struct ClientIds(Vec<spru_bevy::client::component::ClientId>);

#[derive(Debug, Default, prelude::Resource)]
struct ActiveClientId(Option<spru_bevy::client::component::ClientId>);

fn startup(mut commands: prelude::Commands) {
    commands.spawn(bevy::prelude::Camera2d);
    commands.queue(spru_bevy::server::command::Init::<Server, _> {
        game_init: game::Init,
        player_init: player::Init,
        reaction: Reaction,
    })
}

fn print_piles(
    q_piles: prelude::Query<
        (
            &spru_bevy::client::component::ClientId,
            &Item<spru_util::pile::Pile<data::Card>>,
        ),
        (prelude::Changed<Item<spru_util::pile::Pile<data::Card>>>,),
    >,
) {
    for (client_id, pile) in q_piles {
        let mut s = String::new();
        for card in &**pile {
            s.push_str(&format!("{} ", card.face().letters_str()));
        }
        prelude::trace!(name: "pile_changed", client = client_id.into_u32(), value = s);
    }
}

fn misc_input(
    keys: prelude::Res<prelude::ButtonInput<prelude::KeyCode>>,
    mut world_inspector_toggle: prelude::ResMut<WorldInspectorToggle>,
) {
    if keys.just_pressed(prelude::KeyCode::F1) {
        world_inspector_toggle.0 = !world_inspector_toggle.0;
    }
}

#[derive(Debug)]
struct DefaultTrue(bool);

impl Default for DefaultTrue {
    fn default() -> Self {
        Self(true)
    }
}

fn panel_ui(
    mut q_server: prelude::Query<(
        &spru_bevy::common::component::GameId,
        &spru_bevy::server::component::Runner<Server>,
        &spru_bevy::common::component::Root<Common>,
        &mut spru_bevy::server::component::FromUser<Server>,
    )>,
    mut q_client: prelude::Query<(
        &spru_bevy::common::component::GameId,
        &spru_bevy::client::component::ClientId,
        &spru_bevy::client::component::EntityMap,
        &spru_bevy::common::component::Root<Common>,
        &mut spru_bevy::client::component::FromUser<Client>,
    )>,
    q_game_root: prelude::Query<&Item<game::Root>>,
    q_player_map: prelude::Query<&Item<player_map::PlayerMap<crate::player::Root>>>,
    q_pile: prelude::Query<&Item<spru_util::pile::Pile<data::Card>>>,
    q_current_turn: prelude::Query<&Item<spru_util::rotating::Rotating<spru::player::Id>>>,
    q_player_fsm: prelude::Query<&Item<spru_util::fsm::Fsm<player::machine::Impl>>>,
    q_counter: prelude::Query<&Item<spru_util::counter::Counter<u32>>>,
    q_play: prelude::Query<&Item<Play>>,
    game_id: prelude::Res<GameId>,
    client_ids: prelude::Res<ClientIds>,
    log: prelude::Res<Log>,
    mut active_client_id: prelude::ResMut<ActiveClientId>,
    mut contexts: bevy_egui::EguiContexts,

    // Condensed to stay under 16 parameter limit
    (
        mut add_player_string,
        mut play_string,
        mut show_spru_help_window,
        mut show_quibbler_help_window,
    ): (
        prelude::Local<String>,
        prelude::Local<String>,
        prelude::Local<DefaultTrue>,
        prelude::Local<DefaultTrue>,
    ),
) -> prelude::Result {
    use bevy_egui::egui;

    fn render_card(ui: &mut egui::Ui, card: &data::Card, is_unplayed: bool) -> egui::Response {
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

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Spru Help")
        .open(&mut show_spru_help_window.0)
        .default_width(640.)
        .default_pos([32., 64.])
        .resizable([true, false])
        .show(ctx, |ui| {
            let mut help_text = include_str!("../spru_help.txt");
            let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
            ui.add(multiline);
        });

    egui::Window::new("Quibbler Help")
        .open(&mut show_quibbler_help_window.0)
        .default_width(640.)
        .default_pos([756., 64.])
        .resizable([true, false])
        .show(ctx, |ui| {
            let mut help_text = include_str!("../quibbler_help.txt");
            let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
            ui.add(multiline);
        });

    let (_, server_runner, server_root, mut server_from_user) =
        Server::filter_mut(&mut q_server, game_id.get()).ok_or("Server not found")?;
    let server_storage = server_runner.storage();
    let server_game_root = server_storage
        .get(**server_root)
        .map_err(spru::item::storage::Error::into_error)?;
    let server_player_map = server_storage
        .get(server_game_root.players)
        .map_err(spru::item::storage::Error::into_error)?;

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
                        player::Input::new(std::mem::take(&mut *add_player_string));
                    server_from_user.add_player(player_init_in);
                }
            });

            if ui.button("Start game").clicked() {
                server_from_user.manual_trigger(reaction::Trigger::StartGame);
            }
        });
    });

    egui::TopBottomPanel::bottom("player_select").show(ctx, |ui| {
        ui.horizontal(|ui| {
            for client_id in &client_ids.0 {
                if let Ok(player_root) = (**server_player_map).get(**client_id) {
                    let button = egui::Button::selectable(
                        Some(*client_id) == active_client_id.0,
                        &player_root.data.username,
                    );
                    if ui.add(button).clicked() {
                        active_client_id.0 = Some(*client_id);
                    }
                }
            }
        });
    });

    egui::TopBottomPanel::bottom("logs").show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .max_height(64.)
            .max_width(f32::INFINITY)
            .show(ui, |ui| {
                egui::Grid::new("log").striped(true).show(ui, |ui| {
                    for log_line in log.iter_logs(active_client_id.0).rev() {
                        ui.label(log_line);
                        ui.end_row();
                    }
                });
            });
    });

    if let Some(active_client_id) = &mut active_client_id.0 {
        let (_, _, entity_map, root, mut from_user) =
            Client::filter_mut(&mut q_client, game_id.get(), *active_client_id)
                .ok_or("Client not found")?;

        let game_root = q_game_root.get(entity_map[**root])?;
        let discard = q_pile.get(entity_map[game_root.discard])?;
        let player_map = q_player_map.get(entity_map[game_root.players])?;
        let active_player_root = player_map.expect_player(**active_client_id);
        let active_player_username = &*active_player_root.data.username;
        let hand = q_pile.get(entity_map[active_player_root.hand])?;
        let current_turn = q_current_turn.get(entity_map[game_root.current_turn])?;
        let current_player = current_turn.current().map(|p| player_map.expect_player(*p));
        let round = q_counter.get(entity_map[game_root.round])?;

        egui::TopBottomPanel::bottom("player_view").show(ctx, |ui| {
            ui.vertical(|ui| -> prelude::Result {
                ui.heading(format!("{active_player_username}'s view"));
                ui.separator();

                ui.label(format!("Round {} of 8", round.value() + 1));

                if let Some(current_player) = current_player {
                    let current_player_fsm = q_player_fsm.get(entity_map[current_player.fsm])?;
                    
                    let current_player_username = &*current_player.data.username;
                    ui.label(format!("{current_player_username}'s turn ({})", current_player_fsm.current()));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Draw from deck").clicked() {
                        from_user.stage_interaction(interaction::Draw::Deck.into());
                    }
                    if let Some(discard_top) = discard.top() {
                        let button_message = format!("Draw '{}' from discard ({} points)", discard_top.face().letters_str(), discard_top.face().points);
                        if ui.button(button_message).clicked() {
                            from_user.stage_interaction(interaction::Draw::Discard.into());
                        }
                    }
                });

                ui.label("Hand (click to discard)");

                ui.horizontal(|ui| {
                    for card in &**hand {
                        if render_card(ui, card, false).clicked() {
                            from_user.stage_interaction(interaction::Discard::new(card.clone()).into());
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

                        let play = interaction::Play::parsed(hand, &words)
                            .map_err(|c| format!("Can't play '{}', missing '{c}'", String::from_utf8(words).unwrap()))?;
                        
                        from_user.stage_interaction(play.into());
                    }

                    if ui.button("Pass").clicked() {
                        from_user.stage_interaction(interaction::Play::pass().into());
                    }

                    Ok(())
                }).inner?;

                ui.separator();

                ui.horizontal(|ui| -> prelude::Result {
                    for (_player_id, player_root) in player_map.iter() {
                        let player_username = &*player_root.data.username;
                        let player_score = q_counter.get(entity_map[player_root.score])?.value();
                        let player_play = q_play.get(entity_map[player_root.played])?;
                        
                        ui.vertical(|ui| {
                            ui.label(format!("{player_username}: {player_score} points"));
                            ui.horizontal(|ui| {
                                if player_play.word_count() > 0 {
                                    for word in player_play.words() {
                                        for card in word {
                                            render_card(ui, card, false);
                                        }
                                        ui.add_space(24.);
                                    }
                                }

                                if !player_play.is_full() {
                                    for card in player_play.unused() {
                                        render_card(ui, card, true);
                                    }
                                }
                            });
                            if player_play.is_played() {
                                let play_score = player_play.base_score();
                                let word_count = player_play.word_count();
                                let max_word_len = player_play.max_word_len();
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
                    if ui.button("Apply").clicked() {
                        from_user.apply_all_interactions();
                    }
                    if ui.button("Revert").clicked() {
                        from_user.revert_all_interactions();
                    }
                });

                Ok(())
            }).inner
        }).inner?;
    }

    Ok(())
}

fn error_to_console(prelude::In(result): prelude::In<prelude::Result>) {
    if let Err(err) = result {
        prelude::warn!("{err}");
    }
}
