mod host_lobby;
pub use host_lobby::{HostLobby, StartHostLobby, StartGame};

use std::time;

use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::Resource)]
pub struct ExternalIp(ExternalIpInternal);

#[derive(Debug)]
enum ExternalIpInternal {
    Pending(bevy::tasks::Task<Option<String>>),
    Complete(Option<String>),
}

impl Default for ExternalIp {
    fn default() -> Self {
        async fn get_external_ip() -> Result<String, String> {
            let request = ehttp::Request::get("https://myexternalip.com/raw");
            let response = ehttp::fetch_async(request).await?;
            if response.ok && let Some(text) = response.text() {
                Ok(text.to_string())
            } else {
                Err(response.status_text)
            }
        }

        let task = bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                // Give up after so many retries
                for _ in 0..10 {
                    match get_external_ip().await {
                        Ok(ip) => {
                            return Some(ip);
                        }
                        Err(err) => {
                            prelude::warn!("Could not query external ip, retrying in 30 seconds: '{err}'");
                            async_io::Timer::after(time::Duration::from_secs(30)).await;
                        }
                    }
                }

                None
            });

        Self(ExternalIpInternal::Pending(task))
    }
}

impl ExternalIp {
    pub fn get(&mut self) -> Option<&str> {
        self.check_complete();

        if let ExternalIpInternal::Complete(ip) = &self.0 {
            ip.as_ref()
                .map(|s| s.as_str())
        } else {
            None
        }
    }

    fn check_complete(&mut self) {
        if let ExternalIpInternal::Pending(task) = &mut self.0 {
            if let Some(output) = bevy::tasks::block_on(bevy::tasks::poll_once(task)) {
                self.0 = ExternalIpInternal::Complete(output);
            }
        }
    }
}

pub(crate) struct RemoteServer;

impl RemoteServer {
    /// Skip the main menu state if we are a dedicated server without UI
    fn skip_main_menu(
        mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
    ) -> prelude::Result {
        cfg_select! {
            not(feature = "ui") => {
                next_state.set(crate::AppState::DedicatedServerStarting);
            }
            _ => {
                let _ = &mut next_state;
            }
        }
        Ok(())
    }

    fn command_line(
        mut commands: prelude::Commands,
    ) {
        for arg in std::env::args().skip(1) {
            if &arg == "--debug-host" {
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
                            max_players: 4,
                            password: String::new(),
                        }
                    )
                    .queue(
                        spru_bevy::local::command::AddLocalPlayer::<crate::Server, crate::Client>::new_for_entity( 
                            crate::player::Data {
                                username: "Host Player".to_string(),
                            },
                            local_client_entity,
                        )
                    )
                    .queue(
                        spru_bevy::server::remote::command::StartListenerBuilder {
                            keep_alive_interval: Some(time::Duration::from_secs(5)),
                            max_idle_timeout: Some(time::Duration::from_mins(2)),
                            .. Default::default()
                        }.build::<crate::Server>()
                    )
                    ;

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
    }
}

impl prelude::Plugin for RemoteServer {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_plugins(spru_bevy::server::remote::Plugin::<crate::Server>::default())
            .init_resource::<ExternalIp>()
            .add_systems(prelude::Startup, (
                Self::skip_main_menu,
                Self::command_line,
            ))
        ;
    }
}