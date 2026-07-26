use std::{borrow::Cow, iter};

use bevy::{ecs::system::IntoSystem as _, prelude};
use bevy_egui::egui;
use spru_bevy::client::resource::ClientMap;

use crate::plugin::ui;

use super::Validated;

pub(super) struct Plugin;

impl Plugin {
    fn client_select_ui(
        world: &mut prelude::World,
        s_egui: &mut bevy::ecs::system::SystemState<(
            bevy_egui::EguiContexts,
            prelude::ResMut<ui::client::ActiveClient>,
        )>,
    ) -> prelude::Result {
        if let Some(active_game) = world.resource::<ui::ActiveGame>().0 {
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
}

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                Self::client_select_ui.pipe(crate::error_to_console),
            ))
            ;
    }
}

#[derive(Debug)]
struct ConfigManyPlayerPanel {
    username0: Validated<String>,
    usernames: Vec<Validated<String>>,
}

impl Default for ConfigManyPlayerPanel {
    fn default() -> Self {
        Self { 
            username0: Validated::new("Player 1".to_string()),
            usernames: vec![],
        }
    }
}

impl ConfigManyPlayerPanel {
    fn usernames(&self) -> impl Iterator<Item = &Validated<String>> {
        iter::once(&self.username0).chain(&self.usernames)
    }

    fn usernames_mut(&mut self) -> impl Iterator<Item = &mut Validated<String>> {
        iter::once(&mut self.username0).chain(&mut self.usernames)
    }
}

impl super::ConfigPanel for ConfigManyPlayerPanel {
    fn valid(&self) -> Result<(), Cow<'static, str>> {
        let usernames = self.usernames()
            .map(|v| v.text());
        crate::player::validate_usernames(usernames)?;
        Ok(())
    }
    
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Player Names");
            ui.end_row();
            
            let can_remove_player = !self.usernames.is_empty();

            let mut remove_index = None;
            for (i, username) in self.usernames_mut().enumerate() {
                if super::player_row(ui, username, can_remove_player) {
                    remove_index = Some(i);
                }
            }

            let add_button = egui::Button::new("+");
                
            if ui.add(add_button).clicked() {
                self.usernames.push(Validated::new(format!("Player {}", self.usernames.len() + 2)));
            }
            ui.end_row();
            
            if let Some(remove_index) = remove_index {
                let removed = self.usernames.remove(remove_index.saturating_sub(1));
                if remove_index == 0 {
                    self.username0 = removed;
                }
            }
        });
    }
}

#[derive(Debug)]
pub(super) struct ConfigLocal {
    step: usize,
    game_panel: super::ConfigGamePanel,
    player_panel: ConfigManyPlayerPanel,
}

impl Default for ConfigLocal {
    fn default() -> Self {
        Self { 
            step: Default::default(), 
            game_panel: super::ConfigGamePanel::new_local(), 
            player_panel: Default::default(),
        }
    }
}

impl super::Config for ConfigLocal {
    fn title(&self) -> &'static str {
        "Create Hotseat Game"
    }

    fn panels(&mut self) -> (Vec<&mut dyn super::ConfigPanel>, &mut usize) {
        (
            vec![
                &mut self.game_panel as &mut dyn super::ConfigPanel,
                &mut self.player_panel,
            ],
            &mut self.step,
        )
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        let settings = self.game_panel.to_settings();

        let mut entity = commands.spawn_empty();

        entity.queue(move |mut entity: prelude::EntityWorldMut| {
            use prelude::EntityCommand as _;

            entity.reborrow_scope(|entity| {
                spru_bevy::server::command::Init::<crate::Server, crate::GameInit> {
                    game_init: crate::game::init::new(settings),
                    player_init: crate::player::init::new(),
                    reaction: crate::reaction::new(),
                }.apply(entity);
            });
        });

        for username in self.player_panel.usernames().map(|v| v.text().to_string()) {
            entity.queue(spru_bevy::local::command::AddLocalPlayer::<crate::Server, crate::Client>::new( 
                crate::player::Data {
                    username,
                } 
            ));
        }

        entity.queue(spru_bevy::server::command::ManualTrigger::<crate::Server> { 
            trigger: crate::reaction::Trigger::StartGame,
        });

        commands.insert_resource(prelude::NextState::PendingIfNeq(crate::AppState::InGame));
    }
}