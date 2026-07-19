use std::{collections::HashSet, time};

use bevy::prelude;
use bevy_egui::egui;

use super::ConfigPanel as _;

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
    fn show(&mut self, ctx: &mut egui::Context) -> bool {
        self.game_panel.show_panel(&mut self.step, 0, ctx, "Next");
        self.player_panel.show_panel(&mut self.step, 1, ctx, "Next");
        self.server_panel.show_panel(&mut self.step, 2, ctx, "Start");
        self.step == 3
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
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
                    game_init: crate::game::init::new(),
                    player_init: crate::player::init::new(),
                    reaction: crate::reaction::new(),
                }
            )
            .queue(
                crate::plugin::remote_server::StartHostLobby {
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
        #[cfg(feature = "ui")]
        commands.entity(local_client_entity)
            .queue(|mut entity: prelude::EntityWorldMut| {
                if let Ok((&game_id, &client_id)) = entity.get_components::<(&spru_bevy::common::component::GameId, &spru_bevy::client::component::ClientId)>() {
                    entity.resource_scope::<crate::plugin::ui::ActiveClient, ()>(|_, mut active_client| {
                        active_client.set(*game_id, Some(*client_id));
                    });
                } else {
                    prelude::error!("Expected GameId and ClientId");
                }
            });
    }
}