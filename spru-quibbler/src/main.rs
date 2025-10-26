#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod actions;
use std::fmt;

pub use actions::Actions;
pub mod data;
pub mod game;
pub mod interaction;
pub use interaction::Interaction;
pub mod round;
mod play;
pub use play::Play;
mod player;
mod reaction;
pub use reaction::Reaction;
mod state;
use spru_bevy::{client::ClientSSS as _, server::ServerSSS as _};
use spru_util::player_map;
pub use state::State;

use bevy::{ecs::system::IntoSystem, prelude};

type Client = spru::client::ClientImpl<Interaction, game::Outcome>;
type Server = spru::server::ServerImpl<Interaction, Reaction, player::Init>;

fn main() {
    use prelude::PluginGroup as _;

    let _frame_duration = std::time::Duration::from_secs_f32(1. / 30.);

    bevy::app::App::new()
        .add_plugins((
            // bevy::MinimalPlugins.set(
            //     bevy::app::ScheduleRunnerPlugin::run_loop(frame_duration)
            // ),
            bevy::DefaultPlugins.set(bevy::log::LogPlugin {
                filter: "spru=trace,spru_bevy=info,spru_quibbler=trace".to_string(),
                ..Default::default()
            }),
            // bevy::log::LogPlugin {
            //     filter: "spru=trace,spru_bevy=trace,spru_quibbler=trace".to_string(),
            //     .. Default::default()
            // },
            spru_bevy::client::Plugin::<Client>::default(),
            spru_bevy::server::Plugin::<Server>::default(),
            spru_bevy::local::Plugin::<Server, Client>::default(),
            // Not yet updated to 0.17
            // https://github.com/cxreiff/bevy_ratatui
            // bevy_ratatui::RatatuiPlugins::default(),
            bevy_simple_text_input::TextInputPlugin,
            bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ))
        .init_resource::<GameId>()
        .init_resource::<ClientIds>()
        .init_resource::<ActiveClientId>()
        .add_systems(prelude::Startup, (startup,))
        .add_systems(
            prelude::FixedUpdate,
            (process_input.pipe(error_to_console), print_piles),
        )
        // Server Init
        .add_observer(
            |server_init: prelude::On<spru_bevy::server::event::Init<Server>>,
             mut game_id: prelude::ResMut<GameId>,
             mut q_server: prelude::Query<(
                &spru_bevy::common::component::GameId,
                &mut spru_bevy::server::component::FromUser<Server>,
            )>|
             -> prelude::Result {
                let gid = *server_init.result.as_ref().map_err(ToString::to_string)?;
                game_id.set(gid);

                let (_, mut from_user) =
                    Server::filter_mut(&mut q_server, gid).ok_or("Server not found")?;

                for i in 0..2 {
                    from_user.add_player(player::Input {
                        username: format!("Player {i}"),
                    });
                }

                Ok(())
            },
        )
        // Client Init
        .add_observer(
            |client_init: prelude::On<spru_bevy::client::event::Init<Client>>,
             mut client_ids: prelude::ResMut<ClientIds>|
             -> prelude::Result {
                let client_id = *client_init.result.as_ref().map_err(ToString::to_string)?;
                client_ids.push(client_id);
                Ok(())
            },
        )
        .run();
}

#[derive(Debug)]
pub struct Error(pub anyhow::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

macro_rules! anyhow {
    ($msg:literal $(,)?) => {
        $crate::Error(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::Error(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error(anyhow::anyhow!($fmt, $($arg)*))
    }
}
pub(crate) use anyhow;

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::anyhow!($msg).into());
    };
    ($err:expr $(,)?) => {
        return Err($crate::::anyhow!($err).into());
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::::anyhow!($fmt, $($arg)*).into());
    }
}
pub(crate) use bail;

#[derive(Debug, Default, prelude::Resource)]
struct GameId(Option<spru_bevy::common::component::GameId>);

impl GameId {
    pub fn get(&self) -> spru_bevy::common::component::GameId {
        self.0.unwrap()
    }

    fn set(&mut self, game_id: spru_bevy::common::component::GameId) {
        self.0 = Some(game_id);
    }
}

#[derive(Debug, Default, prelude::Resource)]
struct ClientIds(Vec<spru_bevy::client::component::ClientId>);

impl ClientIds {
    pub fn get(&self) -> &[spru_bevy::client::component::ClientId] {
        &self.0
    }

    fn push(&mut self, client_id: spru_bevy::client::component::ClientId) {
        self.0.push(client_id);
    }
}

#[derive(Debug, Default, prelude::Resource)]
struct ActiveClientId(Option<spru_bevy::client::component::ClientId>);

impl ActiveClientId {
    pub fn get(&self) -> Option<spru_bevy::client::component::ClientId> {
        self.0
    }

    fn set(&mut self, client_id: spru_bevy::client::component::ClientId) {
        self.0 = Some(client_id);
    }
}

fn startup(mut commands: prelude::Commands) {
    commands.spawn(bevy::prelude::Camera2d);
    commands.spawn((
        bevy_simple_text_input::TextInput,
        prelude::Node {
            padding: prelude::UiRect::all(prelude::Val::Px(5.0)),
            border: prelude::UiRect::all(prelude::Val::Px(2.0)),
            ..Default::default()
        },
        prelude::BorderColor::all(prelude::Color::BLACK),
    ));
    commands.queue(spru_bevy::server::command::Init::<Server, _> {
        game_init: game::Init,
        player_init: player::Init,
        reaction: Reaction,
    })
}

fn print_piles(
    q_piles: prelude::Query<
        (
            &spru_bevy::client::component::ClientId,
            &spru_bevy::client::component::Item<spru_util::pile::State<data::Card>>,
        ),
        (
            prelude::Changed<
                spru_bevy::client::component::Item<spru_util::pile::State<data::Card>>,
            >,
        ),
    >,
) {
    for (client_id, pile) in q_piles {
        let mut s = String::new();
        for card in &**pile {
            s.push_str(&format!("{} ", card.face().letters));
        }
        prelude::trace!(name: "pile_changed", client = client_id.0.into_u32(), value = s);
    }
}

fn process_input(
    mut events: prelude::MessageReader<bevy_simple_text_input::TextInputSubmitMessage>,
    mut q_server: prelude::Query<(
        &spru_bevy::common::component::GameId,
        &mut spru_bevy::server::component::Runner<Server>,
        &mut spru_bevy::server::component::FromUser<Server>,
    )>,
    mut q_client: prelude::Query<(
        &spru_bevy::common::component::GameId,
        &spru_bevy::client::component::ClientId,
        &spru_bevy::client::component::EntityMap,
        &mut spru_bevy::client::component::FromUser<Client>,
    )>,
    q_player_root: prelude::Query<(
        &spru_bevy::client::component::ClientId,
        &spru_bevy::client::component::Item<player_map::State<crate::player::Root>>,
    )>,
    q_hand: prelude::Query<(
        &spru_bevy::client::component::Item<spru_util::pile::State<data::Card>>,
    )>,
    game_id: prelude::Res<GameId>,
    client_ids: prelude::Res<ClientIds>,
    mut active_client_id: prelude::ResMut<ActiveClientId>,
) -> prelude::Result {
    for event in events.read() {
        let game_id = game_id.get();

        let text = event.value.as_bytes();
        prelude::info!("'{}'", text.escape_ascii());

        let mut words = text.split(u8::is_ascii_whitespace);
        if let Some(command) = words.next() {
            // let args: Vec<_> = words.collect();

            let (_, server_runner, mut server_from_user) =
                Server::filter_mut(&mut q_server, game_id).ok_or("Server not found")?;

            let client_id;
            let client_entity_map;
            let client_from_user;
            if let Some(active_client_id) = active_client_id.get() {
                let (_, _, entity_map, from_user) =
                    Client::filter_mut(&mut q_client, game_id, active_client_id)
                        .ok_or("Client not found")?;
                client_id = Ok(active_client_id);
                client_entity_map = Ok(entity_map);
                client_from_user = Ok(from_user);
            } else {
                client_id = Err("Client context not set");
                client_entity_map = Err("Client context not set");
                client_from_user = Err("Client context not set");
            }

            let player_root = 'player_root: {
                for (&current_client_id, player_root) in q_player_root {
                    if client_id.ok() == Some(current_client_id) {
                        break 'player_root player_root
                            .get(current_client_id.0)
                            .map_err(prelude::BevyError::from);
                    }
                }
                Err("Player root not found".into())
            };

            let hand = 'hand: {
                if let Ok(player_root) = player_root
                    && let Ok(client_entity_map) = client_entity_map
                    && let Ok(hand_entity) = client_entity_map.get(player_root.hand)
                    && let Ok((hand,)) = q_hand.get(hand_entity)
                {
                    break 'hand Some(&**hand);
                }
                None
            }
            .ok_or("Hand not found");

            match command {
                b"add_player" => {
                    let username = words.next().ok_or("Expected username")?;

                    let username = String::from_utf8(username.to_vec()).unwrap();

                    server_from_user.add_player(player::Input { username });
                }
                b"start" => {
                    server_from_user.manual_trigger(reaction::Trigger::StartGame);
                }
                b"save" => {
                    let save = server_runner.save()?;
                    let text =
                        ron::ser::to_string_pretty(&save, ron::ser::PrettyConfig::default())?;
                    arboard::Clipboard::new()?.set_text(text)?;
                }
                b"client" => {
                    let num = words.next().ok_or("Expected ClientId")?;

                    let num: usize = str::from_utf8(num)
                        .unwrap()
                        .parse()
                        .map_err(|_| "Invalid ClientId")?;

                    let client_id = *client_ids.get().get(num).ok_or("ClientId does not exist")?;

                    active_client_id.set(client_id);
                }
                // Client scoped
                b"draw" => {
                    let location = words.next().ok_or("Invalid arguments")?;

                    let interaction: Interaction = match location {
                        b"deck" => Ok(interaction::Draw::Deck.into()),
                        b"discard" => Ok(interaction::Draw::Discard.into()),
                        _ => Err("Invalid location"),
                    }?;

                    client_from_user?.stage_interaction(interaction);
                }
                b"discard" => {
                    let card_letters = words.next().ok_or("Invalid arguments")?;
                    let card_letters = card_letters.to_ascii_uppercase();
                    let card = data::Card::get(&card_letters).ok_or("Invalid card")?;
                    client_from_user?.stage_interaction(interaction::Discard::new(card).into());
                }
                b"play" => {
                    let mut words: Vec<_> = words.collect::<Vec<_>>().join(&b' ');
                    words.make_ascii_uppercase();

                    let interaction = interaction::Play::parsed(hand?, &words)
                        .map_err(|c| format!("No card for character {}", c as char))?;

                    client_from_user?.stage_interaction(interaction.into());
                }
                b"pass" => {
                    client_from_user?.stage_interaction(interaction::Play::pass().into());
                }
                b"apply" => {
                    client_from_user?.apply_all_interactions();
                }
                b"revert" => {
                    client_from_user?.revert_all_interactions();
                }
                _ => return Err("Invalid command".into()),
            }
        }
    }

    Ok(())
}

fn error_to_console(prelude::In(result): prelude::In<prelude::Result>) {
    if let Err(err) = result {
        prelude::warn!("{err}");
    }
}
