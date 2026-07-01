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
        let port = self.server_panel.port.into_value();
        let password = self.server_panel.password;

        // It's not ideal to maintain a separate list of usernames in the connection observer,
        // but it's simpler until we have a fleshed-out lobby. A username conflict also
        // has no impact on correctness (everything is done with ids), it's just confusing.
        let mut claimed_usernames = HashSet::new();
        claimed_usernames.insert(username.clone());

        commands.spawn_empty()
            .queue(
                spru_bevy::server::command::Init::<crate::Server, crate::GameInit> {
                    game_init: crate::game::init::new(),
                    player_init: crate::player::init::new(),
                    reaction: crate::reaction::new(),
                }
            )
            .observe(move |mut attemped_connection: prelude::On<spru_bevy::server::remote::event::AttemptedConnection::<crate::player::Input>>| {
                attemped_connection.propagate(false);
                
                if attemped_connection.headers.get("password").map(String::as_str).unwrap_or("") == &*password {
                    let mut username = attemped_connection.headers
                        .get("username")
                        .map(String::as_str)
                        .unwrap_or("Anonymous");

                    // At least MAX_MAX_PLAYERS fallback usernames provided
                    let fallback_usernames = ["Anonymous2", "Anonymous3", "Anonymous4", "Anonymous5", "Anonymous6", "Anonymous7", "Anonymous8"];
                    let mut iter = fallback_usernames.iter();
                    while claimed_usernames.contains(username) {
                        username = *iter.next()
                            .expect("Not enough fallback usernames");
                    }

                    let username = username.to_string();
                    claimed_usernames.insert(username.clone());
                    
                    attemped_connection.set_response(spru_bevy::server::remote::JoinRequestResponse::AcceptNew(crate::player::Input {
                        username,
                    }));
                } else {
                    attemped_connection.set_response(spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed);
                }
            })
            .queue(
                spru_bevy::local::command::AddLocalPlayer::<crate::Server, crate::Client>::new( 
                    crate::player::Input {
                        username,
                    } 
                )
            )
            .queue(
                spru_bevy::server::remote::command::StartListenerBuilder {
                    port: Some(port),
                    keep_alive_interval: Some(time::Duration::from_secs(5)),
                    max_idle_timeout: Some(time::Duration::from_mins(2)),
                    .. Default::default()
                }.build::<crate::Server>()
            )
        ;
    }
}