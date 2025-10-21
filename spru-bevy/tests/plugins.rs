use std::marker::PhantomData;

use bevy::prelude;

use spru_bevy::{common, server::ServerSSS as _, client::ClientSSS as _};
use spru_test::game::minimal;

fn default_plugins_with_logging() -> bevy::app::PluginGroupBuilder {
    use bevy::app::PluginGroup as _;

    bevy::DefaultPlugins.set(bevy::log::LogPlugin {
        filter: "spru_bevy=trace,plugins=trace".to_string(),
        .. Default::default()
    })
}

#[test]
fn just_plugins() -> impl std::process::Termination {
    

    let exit = bevy::app::App::new()
        .add_plugins((
            default_plugins_with_logging(),
            spru_bevy::client::Plugin::<minimal::Client>::default(),
            spru_bevy::server::Plugin::<minimal::Server>::default(),
            spru_bevy::local::Plugin::<minimal::Server, minimal::Client>::default(),
        ))
        .add_systems(prelude::FixedUpdate, (
            exit_after_delay(2, true),
        ))
        .run();

    exit
}

#[test]
fn local_multiplayer() -> impl std::process::Termination {
    // fn setup(
    //     world: &mut prelude::World,
    //     mut game_id: prelude::Local<Option<spru_bevy::common::component::GameId>>,
    //     mut p0_id: prelude::Local<Option<spru::player::Id>>,
    //     mut p1_id: prelude::Local<Option<spru::player::Id>>,
    //     mut ticks: prelude::Local<u32>,
    // ) {
    //     use bevy::ecs::system::Command as _;
    //     *ticks += 1;
    //     if *ticks == 1 {
    //         let (server_entity, server_game_id) = spru_bevy::server::command::Init::<minimal::Server, _> {
    //             game_init: minimal::GameInit(minimal::LobbyInfo),
    //             player_init: minimal::PlayerInit,
    //             reaction: minimal::Reaction,
    //         }
    //             .apply(world)
    //             .unwrap();

    //         *game_id = Some(server_game_id);
    //         let game_id = server_game_id;

            
    //         *p0_id = Some(spru_bevy::server::command::AddPlayer::<minimal::Server> {
    //             game_id,
    //             player_init_input: spru_test::game::minimal::PlayerColor::Blue,
    //         }
    //             .apply(world)
    //             .unwrap());



    //         *p1_id = Some(spru_bevy::server::command::AddPlayer::<minimal::Server> {
    //             game_id,
    //             player_init_input: spru_test::game::minimal::PlayerColor::Red,
    //         }
    //             .apply(world)
    //             .unwrap());
    //     } else {
    //         let game_id = game_id.unwrap();
    //         let p0_id = p0_id.unwrap();
    //         let p1_id = p1_id.unwrap();

    //         if *ticks == 5 {
    //             let interaction_id = spru_bevy::client::command::StageInteraction::<minimal::Client> {
    //                 game_id,
    //                 client_id: spru_bevy::client::component::ClientId(p1_id),
    //                 interaction: minimal::Interaction,
    //             }
    //                 .apply(world)
    //                 .unwrap();

    //             spru_bevy::client::command::RevertInteractions::<minimal::Client> {
    //                 game_id,
    //                 client_id: spru_bevy::client::component::ClientId(p1_id),
    //                 pending_transaction: Some(interaction_id),
    //                 client: PhantomData,
    //             }
    //                 .apply(world)
    //                 .unwrap();
    //         } else if *ticks == 8 {
    //             let interaction_id = spru_bevy::client::command::StageInteraction::<minimal::Client> {
    //                 game_id,
    //                 client_id: spru_bevy::client::component::ClientId(p1_id),
    //                 interaction: minimal::Interaction,
    //             }
    //                 .apply(world)
    //                 .unwrap();

    //             spru_bevy::client::command::ApplyInteractions::<minimal::Client> {
    //                 game_id,
    //                 client_id: spru_bevy::client::component::ClientId(p1_id),
    //                 pending_transaction: Some(interaction_id),
    //                 client: PhantomData,
    //             }
    //                 .apply(world)
    //                 .unwrap();

    //             prelude::debug!("Setup complete");
    //         }
    //     }
    // }

    // fn check_winner(
    //     q_game_outcome: prelude::Query<(
    //         &mut common::component::GameOutcome<minimal::Common>,
    //     )>,
    //     mut exit: prelude::MessageWriter<prelude::AppExit>,
    // ) {
    //     use bevy::ecs::change_detection::DetectChanges as _;
        
    //     for (game_outcome, ) in q_game_outcome {
    //         if game_outcome.is_changed() && !game_outcome.is_added() {
    //             if let Some(game_outcome) = game_outcome.0.as_ref() {
    //                 prelude::debug!("GameOutcome: {game_outcome:?}");
    //                 exit.write(prelude::AppExit::Success);
    //                 return;
    //             }
    //         }
    //     }
    // }

    fn setup(
        mut commands: prelude::Commands,
    ) {
        commands.queue(spru_bevy::server::command::Init::<minimal::Server, _> {
            game_init: minimal::GameInit(minimal::LobbyInfo),
            player_init: minimal::PlayerInit,
            reaction: minimal::Reaction,
        })
    }

    let exit = bevy::app::App::new()
        .add_plugins((
            default_plugins_with_logging(),
            spru_bevy::client::Plugin::<minimal::Client>::default(),
            spru_bevy::server::Plugin::<minimal::Server>::default(),
            spru_bevy::local::Plugin::<minimal::Server, minimal::Client>::default(),
        ))
        .add_observer(|
            server_init: prelude::On<spru_bevy::server::event::Init<minimal::Server>>, 
            mut q_server: prelude::Query<(
                &spru_bevy::common::component::GameId,
                &mut spru_bevy::server::component::FromUser<minimal::Server>,
            )>,
        | -> prelude::Result {
            let game_id = *server_init.event().result
                .as_ref()
                .map_err(ToString::to_string)?;
            let (_, mut from_user) = minimal::Server::filter_mut(&mut q_server, game_id)
                .ok_or("server not found")?;
            from_user.add_player(minimal::PlayerColor::Blue);
            from_user.add_player(minimal::PlayerColor::Red);
            Ok(())
        })
        .add_observer(|
            client_init: prelude::On<spru_bevy::client::event::Init<minimal::Client>>,
            mut q_client: prelude::Query<(
                &spru_bevy::common::component::GameId,
                &spru_bevy::client::component::ClientId,
                &mut spru_bevy::client::component::FromUser<minimal::Client>,
            )>,
        | -> prelude::Result {
            let game_id = client_init.event().game_id;
            let client_id = *client_init.event().result
                .as_ref()
                .map_err(ToString::to_string)?;
            let (_, _, mut from_user) = minimal::Client::filter_mut(&mut q_client, game_id, client_id)
                .ok_or("Client not found")?;
            from_user.stage_interaction(minimal::Interaction);
            from_user.revert_all_interactions();
            from_user.stage_interaction(minimal::Interaction);
            from_user.apply_all_interactions();
            Ok(())
        })
        .add_observer(|
            game_complete: prelude::On<spru_bevy::server::event::GameComplete<minimal::Server>>,
            mut exit: prelude::MessageWriter<prelude::AppExit>,
        | -> prelude::Result {
            let game_outcome = &game_complete.event().game_outcome;

            prelude::debug!("GameOutcome: {game_outcome:?}");
            exit.write(prelude::AppExit::Success);
            Ok(())
        })
        .add_systems(prelude::FixedUpdate, (
            setup,
            exit_after_delay(5, false),
        ))
        .run();

    exit
}

fn exit_after_delay(seconds: u64, success: bool)
    -> impl Fn(prelude::Res<prelude::Time>, prelude::MessageWriter<prelude::AppExit>)
{
    fn _exit_after_delay(
        seconds: u64,
        success: bool,
        time: prelude::Res<prelude::Time>,
        mut exit: prelude::MessageWriter<prelude::AppExit>,
    ) {
        if time.elapsed().as_secs() >= seconds {
            let code = if success { prelude::AppExit::Success } else { prelude::AppExit::Error(std::num::NonZeroU8::MIN) };
            exit.write(code);
        }
    }

    move |time, exit| _exit_after_delay(seconds, success, time, exit)
}

