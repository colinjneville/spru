#[cfg(feature = "client")]
mod client;

#[cfg(feature = "dedicated-server")]
mod dedicated_server;

#[cfg(feature = "host")]
mod host;

#[cfg(feature = "hotseat")]
mod hotseat;

#[cfg(feature = "join")]
mod join;

#[cfg(feature = "server")]
mod server;

use std::borrow::Cow;
use std::{any, fmt};

use bevy::ecs::schedule::SystemCondition;
use bevy::ecs::system::IntoSystem as _;
use bevy::prelude;
use bevy::state::app::AppExtStates;
use bevy_egui::egui;

use crate::plugin;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[derive(prelude::States, prelude::Reflect)]
enum MenuState {
    #[default]
    None,
    
    Esc,
}

#[derive(Debug, Default)]
#[derive(prelude::Resource)]
pub struct ConfigState(Option<Box<dyn Config>>);

impl ConfigState {
    fn ui(
        mut commands: prelude::Commands,
        mut egui: bevy_egui::EguiContexts,
        mut config_state: prelude::ResMut<Self>,
        mut next_app_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
    ) -> prelude::Result {
        if let Some(config) = &mut config_state.0 {
            let ctx = egui.ctx_mut()?;
            if let Some(outcome) = config.show(ctx) {
                match outcome {
                    ConfigOutcome::Exit => {
                        *config_state = ConfigState::default();
                        next_app_state.set_if_neq(crate::AppState::MainMenu);
                    },
                    ConfigOutcome::Complete => {
                        let config = config_state.0.take().unwrap();
                        config.complete(&mut commands);
                    },
                }
            }
        }

        Ok(())
    }

    fn set<C: Config>(&mut self, config: C) {
        self.0 = Some(Box::new(config));
    }
}

enum ConfigOutcome {
    Exit,
    Complete,
}

trait Config: fmt::Debug + Send + Sync + 'static {
    fn title(&self) -> &'static str;

    // (Slice of all panels, index of active panel)
    fn panels(&mut self) -> (Vec<&mut dyn ConfigPanel>, &mut usize);

    fn complete(self: Box<Self>, commands: &mut prelude::Commands);

    /// Returns true if the user has backed out completely
    fn show(&mut self, ctx: &mut egui::Context)
        -> Option<ConfigOutcome>
    {
        enum Action {
            Back,
            Next,
        }

        let title = self.title();
        let (panels, panel_index) = self.panels(); 

        let panel_count = panels.len();

        let window_center = ctx.content_rect().center();
        let window_size = egui::Vec2::new((320 * panel_count) as f32, 640.);
        let window_rect = egui::Rect::from_center_size(window_center, window_size);

        let id = egui::Id::new(any::TypeId::of::<Self>());

        let mut action = None;
        let action = &mut action;

        egui::Window::new(title)
            .id(id)
            .collapsible(false)
            .fixed_rect(window_rect)
            .show(ctx, |ui| {
                ui.columns(panel_count, |columns| {
                    for (i, panel) in panels.into_iter().enumerate() {
                        let ui = &mut columns[i];
                        let visible = i <= *panel_index;
                        let active = i == *panel_index;

                        let back_text = if i == 0 { "Cancel" } else { "Back" };
                        let next_text = if i + 1 == panel_count { "Start" } else { "Next" };

                        let anim = ui
                            .animate_bool(ui.id().with("anim"), visible);

                        ui.multiply_opacity(anim);

                        if !active {
                            ui.disable();
                        }

                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            panel.show(ui);

                            // Continue building bottom->up
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                                let valid = panel.valid();
                                
                                ui.horizontal(|ui| {
                                    let button_min_size = egui::Vec2::new(ui.available_size().x / 2., 0.);

                                    let back_button = egui::Button::new(back_text)
                                        .min_size(button_min_size);
                                    let action_back = ui.add(back_button)
                                        .clicked()
                                        .then_some(Action::Back);

                                    *action = action.take().or(action_back);

                                    let next_button = egui::Button::new(next_text)
                                        .min_size(button_min_size);
                                    let action_next = ui.add_enabled(valid.is_ok(), next_button)
                                        .clicked()
                                        .then_some(Action::Next);

                                    *action = action.take().or(action_next);
                                });

                                if let Err(err) = valid {
                                    ui.colored_label(egui::Color32::RED, err);
                                    ui.end_row();
                                }
                            });

                            ui.allocate_space(ui.available_size());
                        });

                    }
                });
            });

        if let Some(action) = action {
            match action {
                Action::Back => if *panel_index == 0 {
                        Some(ConfigOutcome::Exit)
                    } else {
                        *panel_index -= 1;
                        None
                    },
                Action::Next => if *panel_index + 1 == panel_count {
                        Some(ConfigOutcome::Complete)
                    } else {
                        *panel_index += 1;
                        None
                    },
            }
        } else {
            None
        }
    }
}

trait ConfigPanel {
    fn show(&mut self, ui: &mut egui::Ui);

    fn valid(&self) -> Result<(), Cow<'static, str>>;
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
    ignore_max_players: bool,
    first_hand: Validated<usize>,
    last_hand: Validated<usize>,
}

impl Default for ConfigGamePanel {
    fn default() -> Self {
        Self { 
            max_players: Validated::new(8),
            // Hacky way to reuse this for both local/remote games
            // There's no reason for a local game to 
            ignore_max_players: false,
            first_hand: Validated::new(3),
            last_hand: Validated::new(10),
        }
    }
}

impl ConfigGamePanel {
    fn new_local() -> Self {
        Self {
            ignore_max_players: true,
            .. Self::default()
        }
    }

    fn to_settings(&self) -> crate::game::Settings {
        crate::game::Settings {
            first_hand: self.first_hand.value,
            last_hand: self.last_hand.value,
        }
    }

    fn validate_max_players(s: &str) -> Result<usize, &'static str> {
        let value = s.parse()
            .map_err(|_| "Max players must be an integer")?;
        (value >= 1 && value <= MAX_MAX_PLAYERS)
            .then_some(value)
            .ok_or("Max players must be between 1 and 8, inclusive")
    }

    fn validate_hand(s: &str) -> Result<usize, &'static str> {
        let value = s.parse()
            .map_err(|_| "Cards must be an integer")?;
        (value >= 2 && value <= 10)
            .then_some(value)
            .ok_or("A hand must be between 2 and 10 cards")
    }
}

impl ConfigPanel for ConfigGamePanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            if !self.ignore_max_players {
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
            }

            ui.horizontal(|ui| {
                ui.label("Cards in First Hand");
                
                if egui::TextEdit::singleline(self.first_hand.text_mut())
                    .show(ui)
                    .response
                    .lost_focus()
                {
                    self.first_hand.validate(Self::validate_hand);
                }
            });
            if let Some(first_hand_error) = self.first_hand.error() {
                ui.colored_label(egui::Color32::RED, first_hand_error);
                ui.end_row();
            }

            ui.horizontal(|ui| {
                ui.label("Cards in Last Hand");
                
                if egui::TextEdit::singleline(self.last_hand.text_mut())
                    .show(ui)
                    .response
                    .lost_focus()
                {
                    self.last_hand.validate(Self::validate_hand);
                }
            });
            if let Some(last_hand_error) = self.last_hand.error() {
                ui.colored_label(egui::Color32::RED, last_hand_error);
                ui.end_row();
            }
        });
    }

    fn valid(&self) -> Result<(), Cow<'static, str>> {
        (self.first_hand.value <= self.last_hand.value)
            .then_some(())
            .ok_or(Cow::Borrowed("The first hand cannot have more cards than the second hand"))
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

fn cast_option<T: Clone + Send + Sync + 'static>(dynamic: rhai::Dynamic) -> Option<T> {
    (!dynamic.is_unit()).then(|| dynamic.cast())
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

    fn menu_none_ui(
        keys: prelude::Res<prelude::ButtonInput<prelude::KeyCode>>,
        mut next_menu_state: prelude::ResMut<prelude::NextState<MenuState>>,
    ) -> prelude::Result {
        if keys.just_pressed(prelude::KeyCode::Escape) {
            next_menu_state.set_if_neq(MenuState::Esc);
        }

        Ok(())
    }

    fn menu_esc_ui(
        mut egui: bevy_egui::EguiContexts,
        keys: prelude::Res<prelude::ButtonInput<prelude::KeyCode>>,
        app_state: prelude::Res<prelude::State<crate::AppState>>,
        mut next_menu_state: prelude::ResMut<prelude::NextState<MenuState>>,
        mut app_exit: prelude::MessageWriter<prelude::AppExit>,
    ) -> prelude::Result {
        let ctx = egui.ctx_mut()?;

        if keys.just_pressed(prelude::KeyCode::Escape) {
            next_menu_state.set_if_neq(MenuState::None);
        }

        let layer = egui::LayerId::new(egui::Order::Foreground, "esc_menu_backdrop".into());

        ctx.layer_painter(layer)
            .rect_filled(ctx.viewport_rect(), 0, egui::Color32::from_black_alpha(128));

        let screen_center = ctx.content_rect().center();
        let width = 280.;

        egui::Window::new("esc_menu_ui")
            .title_bar(false)
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(screen_center)
            .min_width(width)
            .max_width(width)
            .default_width(width)
            .resizable(false)
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(12.))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 12.;

                    if ui.button("Exit").clicked() {
                        app_exit.write(prelude::AppExit::Success);
                    }
                    if ui.button("Return to Game").clicked() {
                        next_menu_state.set_if_neq(MenuState::None);
                    }
                });
            });

        Ok(())
    }

    fn main_menu_ui(
        mut egui: bevy_egui::EguiContexts,
        mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
        mut config_state: prelude::ResMut<ConfigState>,
        mut app_exit: prelude::MessageWriter<prelude::AppExit>,
        #[cfg(feature = "host")]
        mut external_ip: prelude::ResMut<crate::plugin::host::ExternalIp>,
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
}

impl prelude::Plugin for Ui {
    fn build(&self, app: &mut prelude::App) {
        use prelude::IntoScheduleConfigs as _;

        app
            .add_plugins((
                bevy_egui::EguiPlugin::default(),
                bevy_inspector_egui::quick::WorldInspectorPlugin::new()
                    .run_if(bevy::ecs::schedule::common_conditions::resource_equals(WorldInspectorToggle(true))),
                UiGroup,
            ))
            .init_resource::<WorldInspectorToggle>()
            .init_resource::<ActiveGame>()
            .init_resource::<ConfigState>()
            .init_state::<MenuState>()
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                // AppState-based
                (
                    Self::main_menu_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::MainMenu)),
                (
                    ConfigState::ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::Config)),
                (
                    Self::application_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(crate::AppState::InGame)),
                // MenuState-based
                (
                    Self::menu_none_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(MenuState::None)),
                (
                    Self::menu_esc_ui.pipe(crate::error_to_console),
                ).run_if(prelude::in_state(MenuState::Esc)),
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

struct UiGroup;

impl prelude::PluginGroup for UiGroup {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let builder = bevy::app::PluginGroupBuilder::start::<Self>();

        #[cfg(feature = "client")]
        let builder = builder.add(client::Plugin);

        #[cfg(feature = "dedicated-server")]
        let builder = builder.add(dedicated_server::Plugin);

        #[cfg(feature = "host")]
        let builder = builder.add(host::Plugin);

        #[cfg(feature = "hotseat")]
        let builder = builder.add(hotseat::Plugin);

        #[cfg(feature = "join")]
        let builder = builder.add(join::Plugin);

        #[cfg(feature = "server")]
        let builder = builder.add(server::Plugin);

        builder
    }
}
