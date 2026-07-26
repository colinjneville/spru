use bevy::prelude;

pub(crate) struct Server;

impl prelude::Plugin for Server {
    fn build(&self, app: &mut prelude::App) {
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
            .add_observer(
                |game_complete: prelude::On<spru_bevy::server::event::GameCompleted<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let mut log = q_log.get_mut(game_complete.entity).ok();
                    crate::Log::try_log(&mut log, "Game complete");
                    for (id, name, score) in &game_complete.game_outcome.final_scores {
                        crate::Log::try_log(&mut log, format!("{name} ({id}): {score}"));
                    }
                },
            )
            .add_observer(
                |player_added: prelude::On<spru_bevy::server::event::PlayerAdded>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Player {} added", player_added.player_id);
                    let mut log = q_log.get_mut(player_added.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |player_add_error: prelude::On<spru_bevy::server::event::PlayerAddError>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Add player failed: {}", player_add_error.error);
                    let mut log = q_log.get_mut(player_add_error.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |manual_trigger: prelude::On<spru_bevy::server::event::ManualTrigger<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = "Manual trigger successful";
                    let mut log = q_log.get_mut(manual_trigger.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |manual_trigger_error: prelude::On<spru_bevy::server::event::ManualTriggerError<crate::Server>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Manual trigger failed: {}", manual_trigger_error.error);
                    let mut log = q_log.get_mut(manual_trigger_error.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
        ;
    }
}

