#[cfg(feature = "dedicated-server")]
mod dedicated_server;

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "host")]
mod host_lobby;

#[cfg(feature = "hotseat")]
mod hotseat;

#[cfg(feature = "join")]
mod join;

#[cfg(feature = "join")]
mod join_lobby;

use std::borrow::Cow;
use std::{any, cmp, fmt};
use std::collections::{HashMap, HashSet};

use bevy::ecs::schedule::SystemCondition;
#[cfg(feature = "client")]
use bevy::ecs::system::IntoSystem as _;
use bevy::prelude;
use bevy_egui::egui;

#[cfg(feature = "server")]
use spru_bevy::server::resource::ServerMap;

#[cfg(feature = "client")]
use spru_bevy::client::resource::ClientMap;

use crate::plugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelState {
    Hidden,
    Active,
    Inactive,
}

impl PanelState {
    fn from_step(current_step: usize, panel_step: usize) -> Self {
        match current_step.cmp(&panel_step) {
            cmp::Ordering::Less => Self::Hidden,
            cmp::Ordering::Equal => Self::Active,
            cmp::Ordering::Greater => Self::Inactive,
        }
    }
}

#[derive(Debug, Default)]
#[derive(prelude::Resource)]
pub struct ConfigState(Option<Box<dyn Config>>);

impl ConfigState {
    fn ui(
        mut commands: prelude::Commands,
        mut egui: bevy_egui::EguiContexts,
        mut config_state: prelude::ResMut<Self>,
    ) -> prelude::Result {
        if let Some(config) = &mut config_state.0 {
            let ctx = egui.ctx_mut()?;
            if config.show(ctx) {
                let config = config_state.0.take().unwrap();
                config.complete(&mut commands);
            }
        }

        Ok(())
    }

    fn set<C: Config>(&mut self, config: C) {
        self.0 = Some(Box::new(config));
    }
}

trait Config: fmt::Debug + Send + Sync + 'static {
    fn show(&mut self, ctx: &mut egui::Context) -> bool;

    fn complete(self: Box<Self>, commands: &mut prelude::Commands);
}

trait ConfigPanel {
    fn show(&mut self, ui: &mut egui::Ui);

    fn valid(&self) -> Result<(), Cow<'static, str>>;

    /// Returns true if the user has completed this panel
    fn show_panel(&mut self, config_step: &mut usize, panel_step: usize, ctx: &mut egui::Context, next_text: &str) {
        let state = PanelState::from_step(*config_step, panel_step);

        let egui_id = format!("{}Next", any::type_name::<Self>());

        let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
        let mut ui = egui::Ui::new(ctx.clone(), egui_id.clone().into(), builder);

        // egui panels now use ui instead of ctx, but it's not immediately clear
        // how to properly do so when the panels are distributed among different functions.
        // Just use the deprecated functions for now.
        let to_next = egui::Panel::left(any::type_name::<Self>())
            .default_size(320.)
            .resizable(false)
            .show_animated(ctx, state != PanelState::Hidden, |ui| {
                ui.add_enabled_ui(state == PanelState::Active, |ui| {
                    self.show(ui);
                    egui::Panel::bottom(egui_id)
                        .show_inside(ui, |ui| {
                            let next_button = egui::Button::new(next_text);
                                    
                            let is_valid = match self.valid() {
                                Ok(()) => true,
                                Err(err) => {
                                    ui.colored_label(egui::Color32::RED, err);
                                    ui.end_row();

                                    false
                                }
                            };
                            ui.add_enabled(is_valid, next_button).clicked()
                        }).inner
                }).inner
            }).map_or(false, |resp| resp.inner);

        if to_next {
            *config_step += 1;
        }
    }
}


fn player_row(ui: &mut egui::Ui, username: &mut Validated<String>, can_remove_player: bool) -> bool {
    let do_remove = ui.horizontal(|ui| {
        if ui.text_edit_singleline(username.text_mut())
            .lost_focus()
        {
            username.validate(validate_username);
        }
        
        let do_remove = can_remove_player && ui.button("-").clicked();

        ui.end_row();

        do_remove
    }).inner;

    if let Some(error) = username.error() {
        ui.colored_label(egui::Color32::RED, error);
        ui.end_row();
    }

    do_remove
}

/// The maximum number of players we allow creating a server for.
/// The only hard limit here is the number of cards in deck (~110)
const MAX_MAX_PLAYERS: usize = 8;

const DEFAULT_PORT: u16 = 57298;
const MIN_PORT: u16 = 49152;
const MAX_PORT: u16 = 65535;

fn validate_port(s: &str) -> Result<u16, &'static str> {
    if s.is_empty() {
        Ok(DEFAULT_PORT)
    } else {
        let port = s.parse()
            .map_err(|_| "Port must be blank or an integer between 49152 and 65535 inclusive")?;

        (port >= MIN_PORT && port <= MAX_PORT)
            .then_some(port)
            .ok_or("Port must be between 49152 and 65535 inclusive")
    }
}

fn validate_ip(s: &str) -> Result<String, &'static str> {
    cfg_select! {
        feature = "remote-server" => {
            spru_bevy::remote::aeronet_webtransport::wtransport::Identity::self_signed([s])
                .map(|_| s.to_string())
                .map_err(|_| "Invalid address")
        }
        _ => {
            // TODO this and the panel should just be moved somewhere remote-server specific
            Ok(s.to_string())
        }
    }
}

fn validate_username(s: &str) -> Result<String, &'static str> {
    if s.trim_ascii().is_empty() {
        Err("Username must cannot be blank")
    } else {
        Ok(s.to_string())
    }
}

#[derive(Debug)]
struct Validated<T: ToString> {
    text: String,
    value: T,
    last_error: Option<Cow<'static, str>>,
}

impl<T: ToString> Validated<T> {
    pub fn new(value: T) -> Self {
        Self {
            text: value.to_string(),
            value,
            last_error: None,
        }
    }

    pub fn new_with_text_override(value: T, text: String) -> Self {
        Self {
            text,
            value,
            last_error: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn validate<E: Into<Cow<'static, str>>>(&mut self, validator: impl FnOnce(&str) -> Result<T, E>) -> bool {
        match validator(&self.text) {
            Ok(value) => {
                self.value = value;
                self.last_error = None;
                true
            }
            Err(err) => {
                self.text = self.value.to_string();
                self.last_error = Some(err.into());
                false
            }
        }
    }

    pub fn error(&self) -> Option<&str> {
        self.last_error
            .as_ref()
            .map(Cow::as_ref)
    }
}




#[derive(Debug)]
struct ConfigGamePanel {
    max_players: Validated<usize>,
}

impl Default for ConfigGamePanel {
    fn default() -> Self {
        Self { 
            max_players: Validated::new(2),
        }
    }
}

impl ConfigGamePanel {
    fn validate_max_players(s: &str) -> Result<usize, &'static str> {
        let value = s.parse()
            .map_err(|_| "Max players must be an integer")?;
        (value >= 1 && value <= MAX_MAX_PLAYERS)
            .then_some(value)
            .ok_or("Max players must be between 1 and 8, inclusive")
    }
}

impl ConfigPanel for ConfigGamePanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Max Players");
                
                if egui::TextEdit::singleline(self.max_players.text_mut())
                    .show(ui)
                    .response
                    .lost_focus()
                {
                    self.max_players.validate(Self::validate_max_players);
                }
            });
            if let Some(max_players_error) = self.max_players.error() {
                ui.colored_label(egui::Color32::RED, max_players_error);
                ui.end_row();
            }
        });
    }

    fn valid(&self) -> Result<(), Cow<'static, str>> {
        Ok(())
    }
}

#[derive(Debug)]
struct ConfigRemoteCreatePanel {
    external_ip: Validated<String>,
    port: Validated<u16>,
    password: String,
}

impl Default for ConfigRemoteCreatePanel {
    fn default() -> Self {
        Self { 
            external_ip: Validated::new(String::new()),
            port: Validated::new(DEFAULT_PORT),
            password: String::new(),
        }
    }
}

impl ConfigRemoteCreatePanel {
    pub fn new(external_ip: Option<&str>) -> Self {
        Self {
            external_ip: Validated::new(external_ip.unwrap_or("").to_string()),
            .. Default::default()
        }
    } 
}

impl ConfigPanel for ConfigRemoteCreatePanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("External IP");
                if egui::TextEdit::singleline(self.external_ip.text_mut())
                    .hint_text("localhost")
                    .show(ui)
                    .response
                    .lost_focus()
                {
                    self.external_ip.validate(validate_ip);
                }
            }).response.on_hover_text("Leave blank for localhost");

            if let Some(external_ip_error) = self.external_ip.error() {
                ui.colored_label(egui::Color32::RED, external_ip_error);
                ui.end_row();
            }

            ui.horizontal(|ui| {
                ui.label("Port");
                if egui::TextEdit::singleline(self.port.text_mut())
                    .hint_text("Auto")
                    .show(ui)
                    .response
                    .lost_focus()
                {
                    self.port.validate(validate_port);
                }
            });
            
            if let Some(port_error) = self.port.error() {
                ui.colored_label(egui::Color32::RED, port_error);
                ui.end_row();
            }
            ui.horizontal(|ui| {
                ui.label("Password");
                egui::TextEdit::singleline(&mut self.password)
                    .hint_text("Optional")
                    .show(ui);
            });
        });
    }

    fn valid(&self) -> Result<(), Cow<'static, str>> {
        Ok(())
    }
}

#[derive(Debug)]
struct ConfigPlayerPanel {
    username: Validated<String>,
}

impl ConfigPlayerPanel {
    
}

impl Default for ConfigPlayerPanel {
    fn default() -> Self {
        Self { 
            username: Validated::new("Player 1".to_string()), 
        }
    }
}

impl ConfigPanel for ConfigPlayerPanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Local Player Name");
            ui.end_row();
            player_row(ui, &mut self.username, false);
        });
    }

    fn valid(&self) -> Result<(), Cow<'static, str>> {
        crate::player::validate_usernames(std::iter::once(self.username.text()))?;
        Ok(())
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(prelude::SystemSet)]
enum UiPhase {
    Application,
    GameSelect,
    ClientSelect,
    Server,
    Client,
}

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

impl ActiveGame {
    fn auto_select(
        game_list: prelude::Res<plugin::core::GameList>,
        mut active_game: prelude::ResMut<ActiveGame>,
    ) {
        prelude::info!("auto_select");
        if let Some(game_id) = active_game.0 {
            if let Err(_) = game_list.get().binary_search(&game_id) {
                active_game.0 = None;
            }
        }

        if active_game.0 == None && let Some(&first_game_id) = game_list.get().first() {
            prelude::info_once!("Defaulting to {}", first_game_id);
            active_game.0 = Some(first_game_id);
        }
    }
}

#[derive(Default)]
#[derive(prelude::Resource)]
pub(crate) struct ActiveClient(HashMap<spru::game::Id, spru::player::Id>);

impl ActiveClient {
    pub fn get(&self, active_game: spru::game::Id) -> Option<spru::player::Id> {
        self.0.get(&active_game).copied()
    }

    pub fn set(&mut self, active_game: spru::game::Id, value: Option<spru::player::Id>) {
        if let Some(value) = value {
            prelude::trace!("Setting ActiveClient for {active_game} to {value}");
            self.0.insert(active_game, value);
        } else {
            prelude::trace!("Setting ActiveClient for {active_game} to None");
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

    fn main_menu_ui(
        mut egui: bevy_egui::EguiContexts,
        mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
        mut config_state: prelude::ResMut<ConfigState>,
        mut app_exit: prelude::MessageWriter<prelude::AppExit>,
        #[cfg(feature = "host")]
        mut external_ip: prelude::ResMut<crate::plugin::remote_server::ExternalIp>,
    ) -> prelude::Result {
        let ctx = egui.ctx_mut()?;

        egui::Window::new("Main Menu")
            .open(&mut true)
            .resizable([false, false])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    #[cfg(feature = "hotseat")]
                    {
                        if ui.button("Local Game").clicked() {
                            config_state.set(hotseat::ConfigLocal::default());
                            next_state.set_if_neq(crate::AppState::Config);
                        }
                        ui.end_row();
                    }
                    #[cfg(feature = "host")]
                    {
                        if ui.button("Host Game").clicked() {
                            config_state.set(host::ConfigHost::new(external_ip.get()));
                            next_state.set_if_neq(crate::AppState::Config);
                        }
                        ui.end_row();
                    }
                    #[cfg(feature = "join")]
                    {
                        if ui.button("Join Game").clicked() {
                            config_state.set(join::ConfigJoin::default());
                            next_state.set_if_neq(crate::AppState::Config);
                        }
                        ui.end_row();
                    }
                    #[cfg(all(feature = "dedicated-server"))]
                    {
                        if ui.button("Dedicated Server").clicked() {
                            config_state.set(dedicated_server::ConfigDedicatedServer::default());
                            next_state.set_if_neq(crate::AppState::Config);
                        }
                        ui.end_row();
                    }
                    {
                        if ui.button("Exit").clicked() {
                            app_exit.write_default();
                        }
                        ui.end_row();
                    }
                });
            });

        Ok(())
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
                let mut help_text = include_str!("../../../spru_help.txt");
                let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
                ui.add(multiline);
            });

        egui::Window::new("Quibbler Help")
            .open(&mut show_quibbler_help_window.0)
            .default_width(640.)
            .default_pos([756., 64.])
            .resizable([true, false])
            .show(ctx, |ui| {
                let mut help_text = include_str!("../../../quibbler_help.txt");
                let multiline = egui::TextEdit::multiline(&mut help_text).interactive(false);
                ui.add(multiline);
            });

        Ok(())
    }

    fn game_select_ui(
        mut egui: bevy_egui::EguiContexts,
        game_list: prelude::Res<plugin::core::GameList>,
        mut active_game: prelude::ResMut<ActiveGame>,
    ) -> prelude::Result {
        if game_list.get().len() > 1 {
            let ctx = egui.ctx_mut()?;
            let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
            let mut ui = egui::Ui::new(ctx.clone(), "game_select".into(), builder);

            egui::Panel::bottom("game_select").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for &game_id in game_list.get() {
                            let button = egui::Button::selectable(
                                Some(game_id) == active_game.0,
                                &game_id.friendly_display().to_string(),
                            );
                            if ui.add(button).clicked() {
                                active_game.0 = Some(game_id);
                            }
                        }
                    });
                });
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

            let (mut egui, mut active_client) = s_egui.get_mut(world)?;
            let ctx = egui.ctx_mut()?;
            let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
            let mut ui = egui::Ui::new(ctx.clone(), "player_select".into(), builder);
            ui.set_clip_rect(ctx.content_rect());
            ui
                .response()
                .widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Panel));
            
            if player_names.len() > 1 {
                player_names.sort_unstable_by_key(|(id, _)| *id);

                egui::Panel::bottom("player_select").show(ctx, |ui| {
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
            }
        }

        Ok(())
    }

    #[cfg(feature = "server")]
    fn server_ui_control(
        mut egui: bevy_egui::EguiContexts,
        mut commands: prelude::Commands,
        server_map: prelude::Res<ServerMap>,
        mut add_player_string: prelude::Local<String>,
        active_game: prelude::Res<ActiveGame>,
        
    ) -> prelude::Result {
        let Some(active_game_id) = active_game.0 else {
            return Ok(());
        };
        let Some(server_entity) = server_map.get(active_game_id) else {
            return Ok(());
        };

        let ctx = egui.ctx_mut()?;
        let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
        let mut ui = egui::Ui::new(ctx.clone(), "server_control".into(), builder);
        
        egui::Panel::top("server_control").show(ctx, |ui| {
            ui.vertical(|ui| {
                #[cfg(feature = "local")]
                ui.horizontal(|ui| {
                    let response = ui
                        .button("Add local player");

                    let confirmed = ui
                        .text_edit_singleline(&mut *add_player_string)
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if (response.clicked() || confirmed) && !add_player_string.is_empty() {
                        let player_init_in =
                            crate::player::Data {
                                username: std::mem::take(&mut *add_player_string),
                            };
                        commands
                            .entity(server_entity)
                            .queue(spru_bevy::local::command::AddLocalPlayer::<crate::Server, crate::Client>::new(player_init_in));
                    }
                }).response.on_hover_text("Input player display name");

                if ui.button("Start game").clicked() {
                    commands
                        .entity(server_entity)
                        .queue(spru_bevy::server::command::ManualTrigger::<crate::Server>::new(crate::reaction::Trigger::StartGame));
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

    #[cfg(feature = "client")]
    fn client_ui_player_view(
        ui_data: prelude::In<Option<UiData>>,
        mut commands: prelude::Commands,
        mut egui: bevy_egui::EguiContexts,
        mut play_string: prelude::Local<String>,
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
        let builder = egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.content_rect());
        let mut ui = egui::Ui::new(ctx.clone(), "player_view".into(), builder);
        
        egui::Panel::bottom("player_view").show(ctx, |ui| {
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
                        if render_card(ui, card, false).clicked() {
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
}

impl prelude::Plugin for Ui {
    fn build(&self, app: &mut prelude::App) {
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
            .init_resource::<ConfigState>()
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
                (
                    Self::main_menu_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::MainMenu)),
                (
                    ConfigState::ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::Config)),
                (
                    #[cfg(feature = "join")]
                    join_lobby::join_lobby_connecting_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::Connecting)),
                (
                    #[cfg(feature = "join")]
                    join_lobby::join_lobby_ui_get_client
                        .pipe(join_lobby::join_lobby_ui_get_data)
                        .pipe(join_lobby::join_lobby_ui),
                    #[cfg(feature = "host")]
                    host_lobby::host_lobby_ui,
                ).run_if(prelude::in_state(crate::AppState::InLobby)),
                (
                    Self::application_ui.pipe(crate::error_to_console).in_set(UiPhase::Application),
                    Self::game_select_ui.pipe(crate::error_to_console).in_set(UiPhase::GameSelect),
                    #[cfg(feature = "client")]
                    Self::client_select_ui.pipe(crate::error_to_console).in_set(UiPhase::ClientSelect),
                    #[cfg(feature = "server")]
                    (Self::server_ui_control.pipe(crate::error_to_console), Self::server_ui_log.pipe(crate::error_to_console)).in_set(UiPhase::Server),
                    #[cfg(feature = "client")]
                    Self::client_ui.pipe(crate::error_to_console).in_set(UiPhase::Client),
                ).run_if(prelude::in_state(crate::AppState::InGame)),
            ))
            .add_systems(prelude::Startup, Self::startup)
            .add_systems(prelude::PreUpdate, (
                ActiveGame::auto_select.run_if(
                    prelude::resource_changed::<ActiveGame>
                        .or_eager(prelude::resource_changed::<plugin::core::GameList>)),
            ))
            .add_systems(prelude::Update, Self::misc_input)
        ;

        // panel_ui.pipe(error_to_console),
    }
}

