use bevy::prelude;

pub(crate) struct Server;

impl Server {
    fn startup(mut commands: prelude::Commands) {
        commands
            .spawn_empty()
            .queue(spru_bevy::server::command::Init::<crate::Server, _> {
                game_init: crate::game::init::new(),
                player_init: crate::player::init::new(),
                reaction: crate::reaction::new(),
            });
    }
}

impl prelude::Plugin for Server {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins((
                spru_bevy::server::Plugin::<crate::Server>::default(),
            ))
            // .add_systems(prelude::Startup, Self::startup)
            .add_observer(|
                    server_add: prelude::On<prelude::Add, spru_bevy::server::component::Runner<crate::Server>>,
                    mut commands: prelude::Commands,
                | {
                    commands.entity(server_add.entity)
                        .insert((
                            crate::Log::default(),
                        ));
                }
            )
            .add_observer(|
                    server_remove: prelude::On<prelude::Remove, spru_bevy::server::component::Runner<crate::Server>>,
                    mut commands: prelude::Commands,
                | { 
                    commands.entity(server_remove.entity)
                        .remove::<(
                            crate::Log,
                        )>();
                }
            )
            // Server Init
            // .add_observer(
            //     |server_init: prelude::On<spru_bevy::server::event::Init<crate::Server>>,
            //     mut game_id: prelude::ResMut<GameId>,
            //     mut q_server: prelude::Query<(
            //         &spru_bevy::common::component::GameId,
            //         &mut spru_bevy::server::component::FromUser<crate::Server>,
            //     )>|
            //     -> prelude::Result {
            //         let server_info = *server_init.result.as_ref().map_err(ToString::to_string)?;
            //         game_id.set(server_info.game_id);

            //         let (_, mut from_user) = q_server.get_mut(server_info.entity).map_err(|_| "Server not found")?;

            //         for username in ["Alice", "Bob"] {
            //             from_user.add_player(crate::player::Input {
            //                 username: username.to_string(),
            //             });
            //         }

            //         Ok(())
            //     },
            // )
            .add_observer(
                |game_complete: prelude::On<spru_bevy::server::event::GameComplete<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let mut log = q_log.get_mut(game_complete.entity).ok();
                    crate::Log::try_log(&mut log, "Game complete");
                    for (id, score) in &game_complete.game_outcome.final_scores {
                        crate::Log::try_log(&mut log, format!("{id}: {score}"));
                    }
                },
            )
            .add_observer(
                |add_player: prelude::On<spru_bevy::server::event::AddPlayer<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = match &add_player.result {
                        Ok(player_id) => format!("Player {player_id} added"),
                        Err(err) => format!("Add player failed: {err}"),
                    };
                    let mut log = q_log.get_mut(add_player.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |manual_trigger: prelude::On<spru_bevy::server::event::ManualTrigger<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = match &manual_trigger.result {
                        Ok(()) => "Manual trigger successful".to_string(),
                        Err(err) => format!("Manual trigger failed: {err}"),
                    };
                    let mut log = q_log.get_mut(manual_trigger.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
        ;
    }
}

