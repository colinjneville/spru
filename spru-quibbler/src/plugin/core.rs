use std::{collections::HashSet, time};

#[cfg(any(feature = "client", feature = "server"))]
use bevy::ecs::schedule::SystemCondition as _;
use bevy::{ecs::{change_detection::DetectChangesMut, schedule::IntoScheduleConfigs}, prelude, state::app::AppExtStates as _};

pub(crate) struct Core;

impl Core {
    fn update_game_list(
        #[cfg(feature = "server")]
        server_map: prelude::Res<spru_bevy::server::resource::ServerMap>,
        #[cfg(feature = "client")]
        client_map: prelude::Res<spru_bevy::client::resource::ClientMap>,
        
        game_list: prelude::ResMut<GameList>,
    ) {
        prelude::info!("update_game_list");
        let mut game_ids = HashSet::<spru::game::Id>::new();

        #[cfg(feature = "server")]
        game_ids.extend(server_map.iter().map(|(game_id, _)| game_id));

        #[cfg(feature = "client")]
        game_ids.extend(client_map.iter().map(|(game_id, _, _)| game_id));

        let mut game_ids: Vec<_> = game_ids.into_iter().collect();
        game_ids.sort_unstable();

        game_list
            .map_unchanged(|game_list| &mut game_list.0)
            .set_if_neq(game_ids);

    }

    fn trace_app_state_change(
        mut reader: prelude::MessageReader<prelude::StateTransitionEvent<crate::AppState>>,
    ) {
        for message in reader.read() {
            prelude::info!(?message.exited, ?message.entered);
        }
    }
}

impl prelude::Plugin for Core {
    fn build(&self, app: &mut prelude::App) {
        use prelude::PluginGroup as _;

        let _frame_duration = std::time::Duration::from_secs_f32(1. / 30.);

        let update_game_list_condition = prelude::run_once;

        #[cfg(feature = "client")]
        let update_game_list_condition = update_game_list_condition
            .or_eager(prelude::resource_changed::<spru_bevy::client::resource::ClientMap>);

        #[cfg(feature = "server")]
        let update_game_list_condition = update_game_list_condition
            .or_eager(prelude::resource_changed::<spru_bevy::server::resource::ServerMap>);

        app
            .add_plugins((
                bevy::DefaultPlugins.set(bevy::log::LogPlugin {
                    filter: "spru=info,spru_bevy=trace,spru_quibbler=trace,aeronet_webtransport=debug".to_string(),
                    ..Default::default()
                }),
            ))
            .insert_resource(bevy::winit::WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Continuous,
                unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(
                    time::Duration::from_millis(10),
                ),
            })
            .insert_resource(GameList::default())
            .init_state::<crate::AppState>()
            .add_systems(
                prelude::PreUpdate,
                Self::update_game_list.run_if(update_game_list_condition),
            )
            .add_systems(
                prelude::Update,
                (
                    Self::trace_app_state_change,
                )
            )
            ;
    }
}

#[derive(Debug, Default)]
#[derive(prelude::Resource)]
pub struct GameList(Vec<spru::game::Id>);

impl GameList {
    pub fn get(&self) -> &[spru::game::Id] {
        self.0.as_slice()
    }
}

// This deref impl causes an overflow for the trait solver
// https://github.com/rust-lang/rust/issues/118476
// impl ops::Deref for GameList {
//     type Target = [spru::game::Id];

//     fn deref(&self) -> &Self::Target {
//         self.0.as_slice()
//     }
// }
