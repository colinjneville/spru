use std::{borrow::Cow, iter};

use bevy::prelude;
use bevy_egui::egui;

use super::ConfigPanel as _;

use super::Validated;

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

#[derive(Debug, Default)]
pub(super) struct ConfigLocal {
    step: usize,
    player_panel: ConfigManyPlayerPanel,
}

impl super::Config for ConfigLocal {
    fn show(&mut self, ctx: &mut egui::Context) -> bool {
        self.player_panel.show_panel(&mut self.step, 0, ctx, "Next");

        self.step == 1
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        let mut entity = commands.spawn_empty();

        entity.queue(move |mut entity: prelude::EntityWorldMut| {
            use prelude::EntityCommand as _;

            entity.reborrow_scope(|entity| {
                spru_bevy::server::command::Init::<crate::Server, crate::GameInit> {
                    game_init: crate::game::init::new(),
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