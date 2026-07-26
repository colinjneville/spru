mod host_lobby;

use std::time;

use bevy::{ecs::system::IntoSystem as _, prelude};

use crate::plugin::ui;

pub(super) struct Plugin;

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                host_lobby::ui.pipe(crate::error_to_console),
            ))
            ;
    }
}

#[derive(Debug, Default)]
pub(super) struct ConfigHost {
    step: usize,
    game_panel: super::ConfigGamePanel,
    player_panel: super::ConfigPlayerPanel,
    server_panel: super::ConfigRemoteCreatePanel,
}

impl ConfigHost {
    pub fn new(external_ip: Option<&str>) -> Self {
        Self {
            step: 0,
            game_panel: Default::default(),
            player_panel: Default::default(),
            server_panel: super::ConfigRemoteCreatePanel::new(external_ip),
        }
    }
}

impl super::Config for ConfigHost {
    fn title(&self) -> &'static str {
        "Host Remote Game"
    }

    fn panels(&mut self) -> (Vec<&mut dyn super::ConfigPanel>, &mut usize) {
        (
            vec![
                &mut self.game_panel as &mut dyn super::ConfigPanel,
                &mut self.player_panel,
                &mut self.server_panel,
            ],
            &mut self.step,
        )
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        let settings = self.game_panel.to_settings();
        let max_players = self.game_panel.max_players.into_value();
        let username = self.player_panel.username.into_value();
        let external_ip = self.server_panel.external_ip.into_value();
        let port = self.server_panel.port.into_value();
        let password = self.server_panel.password;

        let mut ips = vec!["localhost", "127.0.0.1", "::1"];
        if !external_ip.is_empty() {
            ips.push(&external_ip);
        }

        let identity = spru_bevy::remote::aeronet_webtransport::wtransport::Identity::self_signed(ips)
            .expect("external_ip should already have been validated");

        let local_client_entity = commands.spawn_empty().id();

        commands.spawn_empty()
            .queue(
                spru_bevy::server::command::Init::<crate::Server, crate::GameInit> {
                    game_init: crate::game::init::new(settings),
                    player_init: crate::player::init::new(),
                    reaction: crate::reaction::new(),
                }
            )
            .queue(
                crate::plugin::host::StartHostLobby {
                    max_players,
                    password,
                }
            )
            .queue(
                spru_bevy::local::command::AddLocalPlayer::<crate::Server, crate::Client>::new_for_entity( 
                    crate::player::Data {
                        username: username.clone(),
                    },
                    local_client_entity,
                )
            )
            .queue(
                spru_bevy::server::remote::command::StartListenerBuilder {
                    identity: Some(identity),
                    port: Some(port),
                    keep_alive_interval: Some(time::Duration::from_secs(5)),
                    max_idle_timeout: Some(time::Duration::from_mins(2)),
                    .. Default::default()
                }.build::<crate::Server>()
            )
        ;

        // Set ActiveClient context to local player
        commands.entity(local_client_entity)
            .queue(|mut entity: prelude::EntityWorldMut| {
                if let Ok((&game_id, &client_id)) = entity.get_components::<(&spru_bevy::common::component::GameId, &spru_bevy::client::component::ClientId)>() {
                    entity.resource_scope::<ui::client::ActiveClient, ()>(|_, mut active_client| {
                        active_client.set(*game_id, Some(*client_id));
                    });
                } else {
                    prelude::error!("Expected GameId and ClientId");
                }
            });
    }
}