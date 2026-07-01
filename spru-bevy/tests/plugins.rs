use bevy::prelude;

use spru_test::game::minimal;

fn headless_plugins_with_logging() -> bevy::app::PluginGroupBuilder {
    use bevy::app::PluginGroup as _;

    bevy::MinimalPlugins
        .set(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add(bevy::log::LogPlugin {
            filter: "spru_bevy=trace,plugins=trace".to_string(),
            ..Default::default()
        })
}

#[test]
fn just_plugins() -> impl std::process::Termination {
    let exit = prelude::App::new()
        .add_plugins((
            headless_plugins_with_logging(),
            spru_bevy::client::Plugin::<minimal::MyClient>::default(),
            spru_bevy::server::Plugin::<minimal::MyServer>::default(),
            spru_bevy::local::Plugin::<minimal::MyServer, minimal::MyClient>::default(),
        ))
        .add_systems(prelude::FixedUpdate, (exit_after_delay(2, true),))
        .run();

    exit
}

#[test]
fn local_multiplayer() -> impl std::process::Termination {
    fn setup(mut commands: prelude::Commands) {
        commands
            .spawn_empty()
            .queue(spru_bevy::server::command::Init::<minimal::MyServer, _> {
                game_init: minimal::GameInit(minimal::LobbyInfo),
                player_init: minimal::MyPlayerInit,
                reaction: minimal::MyReaction,
            });
    }

    let exit = prelude::App::new()
        .add_plugins((
            headless_plugins_with_logging(),
            spru_bevy::client::Plugin::<minimal::MyClient>::default(),
            spru_bevy::server::Plugin::<minimal::MyServer>::default(),
            spru_bevy::local::Plugin::<minimal::MyServer, minimal::MyClient>::default(),
        ))
        .add_observer(
            |
                server_init: prelude::On<spru_bevy::server::event::Init<minimal::MyServer>>, 
                mut commands: prelude::Commands,
            | -> prelude::Result {
                commands.entity(server_init.entity)
                    .queue(spru_bevy::local::command::AddLocalPlayer::<minimal::MyServer, minimal::MyClient>::new(minimal::PlayerColor::Blue))
                    .queue(spru_bevy::local::command::AddLocalPlayer::<minimal::MyServer, minimal::MyClient>::new(minimal::PlayerColor::Red));
                
                Ok(())
            },
        )
        .add_observer(|
                client_init: prelude::On<spru_bevy::client::event::Init<minimal::MyClient>>,
                mut commands: prelude::Commands,
            | -> prelude::Result {                
                commands.entity(client_init.entity)
                    .queue(spru_bevy::client::command::StageInteraction::<minimal::MyClient>::new(minimal::Interaction))
                    .queue(spru_bevy::client::command::RevertInteractions::<minimal::MyClient>::all())
                    .queue(spru_bevy::client::command::StageInteraction::<minimal::MyClient>::new(minimal::Interaction))
                    .queue(spru_bevy::client::command::ApplyInteractions::<minimal::MyClient>::all());

                Ok(())
            },
        )
        .add_observer(
            |game_complete: prelude::On<
                spru_bevy::server::event::GameComplete<minimal::MyServer>,
            >,
             mut exit: prelude::MessageWriter<prelude::AppExit>|
             -> prelude::Result {
                let game_outcome = &game_complete.event().game_outcome;

                prelude::debug!("GameOutcome: {game_outcome:?}");
                exit.write(prelude::AppExit::Success);
                Ok(())
            },
        )
        .add_systems(prelude::FixedUpdate, (setup, exit_after_delay(5, false)))
        .run();

    exit
}

fn exit_after_delay(
    seconds: u64,
    success: bool,
) -> impl Fn(prelude::Res<prelude::Time>, prelude::MessageWriter<prelude::AppExit>) {
    fn _exit_after_delay(
        seconds: u64,
        success: bool,
        time: prelude::Res<prelude::Time>,
        mut exit: prelude::MessageWriter<prelude::AppExit>,
    ) {
        if time.elapsed().as_secs() >= seconds {
            let code = if success {
                prelude::AppExit::Success
            } else {
                prelude::AppExit::Error(std::num::NonZeroU8::MIN)
            };
            exit.write(code);
        }
    }

    move |time, exit| _exit_after_delay(seconds, success, time, exit)
}
