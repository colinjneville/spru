use std::collections::HashMap;

use bevy::prelude;

pub struct StartHostLobby {
    pub max_players: usize,
    pub password: String,
}

impl prelude::EntityCommand for StartHostLobby {
    type Out = ();

    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            max_players,
            password,
        } = self;

        entity
            .insert(HostLobby {
                players: vec![],
                max_players,
                password,
                started: false,
                reconnect_tokens: HashMap::new(),
            })
            .observe(HostLobby::on_connection_attempted)
            .observe(HostLobby::on_connection_accepted)
            .observe(HostLobby::on_disconnected)
            .observe(HostLobby::on_player_added)
            .observe(HostLobby::on_player_removed)
            .world_scope(|world| {
                world.insert_resource(prelude::NextState::PendingIfNeq(crate::AppState::InLobby));
            })
            ;
    }
}

#[derive(Debug)]
pub(crate) struct LobbyPlayer {
    pub(crate) entity: Option<prelude::Entity>,
    pub(crate) player_id: spru::player::Id,
    pub(crate) username: String,
}

#[derive(Debug)]
#[derive(prelude::Component)]
pub struct HostLobby {
    pub players: Vec<LobbyPlayer>,
    pub max_players: usize,
    pub password: String,
    pub started: bool,

    reconnect_tokens: HashMap<crate::player::ReconnectToken, spru::player::Id>,
}

impl HostLobby {
    fn on_connection_attempted(
        mut connection_attempted: prelude::On<spru_bevy::server::remote::event::ConnectionAttempted<crate::player::Data>>,
        q_host_lobby: prelude::Query<(
            &HostLobby,
        )>,
    ) { 
        let Ok((host_lobby, )) = q_host_lobby.get(connection_attempted.server_entity) else {
            prelude::error!("HostLobby not found");
            return;
        };

        let mut username = connection_attempted.headers
            .get("username")
            .map(String::as_str)
            .unwrap_or("Anonymous");
        let password = connection_attempted.headers
            .get("password")
            .map(String::as_str)
            .unwrap_or("");
        let reconnect_token = match crate::player::ReconnectToken::from_headers(&connection_attempted.headers) {
            Ok(token) => token,
            Err(err) => {
                prelude::warn!("Invalid reconnect token: {err}");
                None
            }
        };        

        // First attempt to reconnect a dropped connection, since this can happen whether the game is started or not.
        if let Some(reconnect_token) = &reconnect_token {
            if let Some(player_id) = host_lobby.reconnect_tokens.get(reconnect_token) {
                // TODO Should we close any existing connection for this player? Does it matter?
                connection_attempted.set_response(spru_bevy::server::remote::JoinRequestResponse::AcceptReconnect(*player_id));
                return;
            }
        }

        if host_lobby.started {
            connection_attempted.set_response(spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed("The game has already started".to_string()));
            return;
        }

        if host_lobby.players.len() >= host_lobby.max_players {
            connection_attempted.set_response(spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed("Max player limit reached".to_string()));
            return;
        }

        if password != &*host_lobby.password {
            connection_attempted.set_response(spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed("Invalid password".to_string()));
            return;
        }

        // At least MAX_MAX_PLAYERS fallback usernames provided
        let fallback_usernames = ["Anonymous2", "Anonymous3", "Anonymous4", "Anonymous5", "Anonymous6", "Anonymous7", "Anonymous8"];
        let mut iter = fallback_usernames.iter();
        while host_lobby.players.iter().any(|lobby_player| lobby_player.username == username) {
            username = *iter.next()
                .expect("Not enough fallback usernames");
        }

        let username = username.to_string();
        
        let data = crate::player::Data {
            username,
        };
        
        connection_attempted.set_response(spru_bevy::server::remote::JoinRequestResponse::AcceptNew(data));
    }

    fn on_player_added(
        player_added: prelude::On<spru_bevy::server::event::PlayerAdded>,
        mut q_host_lobby: prelude::Query<(
            &mut HostLobby,
            &spru_bevy::server::component::Runner<crate::Server>,
        )>,
    ) {
        let player_id = player_added.player_id;

        let _entered = prelude::error_span!("HostLobby::on_player_added", %player_id);

        prelude::info!("Player added");

        let Ok((mut host_lobby, runner, )) = q_host_lobby.get_mut(player_added.entity) else {
            prelude::error!("HostLobby not found");
            return;
        };

        use spru_script::DialectEval as _;

        let root = runner.root();

        let username = crate::Language::default()
            .eval(runner.storage(), root, "context.root.players.get(args).data.username", player_id)
            .unwrap();

        host_lobby.players.push(LobbyPlayer {
            entity: None,
            player_id,
            username,
        });
    }

    fn on_connection_accepted(
        connection_accepted: prelude::On<spru_bevy::server::remote::event::ConnectionAccepted>,
        mut q_host_lobby: prelude::Query<(
            &mut HostLobby,
        )>,
    ) {
        let Ok((mut host_lobby, )) = q_host_lobby.get_mut(connection_accepted.server_entity) else {
            prelude::error!("HostLobby not found");
            return;
        };
        
        // Associate the player id now that spru has assigned one
        for lobby_player in &mut host_lobby.players {
            if lobby_player.player_id == connection_accepted.player_id {
                lobby_player.entity = Some(connection_accepted.client_entity);
                break;
            }
        }

        // If the add was successful, register the reconnect_token if provided
        let reconnect_token = match crate::player::ReconnectToken::from_headers(&connection_accepted.headers) {
            Ok(token) => token,
            Err(err) => {
                prelude::warn!("Invalid reconnect token: {err}");
                None
            }
        };

        if let Some(reconnect_token) = reconnect_token {
            host_lobby.reconnect_tokens.insert(reconnect_token.clone(), connection_accepted.player_id);
        }
    }

    fn on_disconnected(
        disconnected: prelude::On<spru_bevy::server::remote::event::RemotePlayerDisconnected>,
        mut commands: prelude::Commands,
        q_host_lobby: prelude::Query<(
            &spru_bevy::common::component::GameId,
            &HostLobby,
        )>,
    ) -> prelude::Result {
        let server_entity = disconnected.server_entity;
        let player_id = disconnected.player_id;
        
        let span = prelude::error_span!("HostLobby::on_disconnect", game_id = bevy::log::tracing::field::Empty, %player_id).entered();
        
        let (game_id, host_lobby, ) = q_host_lobby.get(server_entity)?;
        let game_id = **game_id;

        span.record("game_id", bevy::log::tracing::field::display(game_id));

        prelude::debug!("Remote client disconnected: {:?}", disconnected.reason);

        // If we are still in the lobby, just boot anyone who disconnects
        if !host_lobby.started {
            commands
                .entity(server_entity)
                .queue(spru_bevy::server::command::RemovePlayer::<crate::Server>::new(player_id))
                ;
        }

        Ok(())
    }

    fn on_player_removed(
        player_removed: prelude::On<spru_bevy::server::event::PlayerRemoved>,
        mut q_host_lobby: prelude::Query<(
            &mut HostLobby,
        )>,
    ) {
        let Ok((mut host_lobby, )) = q_host_lobby.get_mut(player_removed.entity) else {
            prelude::error!("HostLobby not found");
            return;
        };

        // Remove any corresponding reconnect token
        host_lobby.reconnect_tokens.retain(|_, player_id| player_removed.player_id != *player_id);

        // Remove player from lobby list
        let disconnected_player = host_lobby.players
            .extract_if(.., |lobby_player| lobby_player.player_id == player_removed.player_id);

        for lobby_player in disconnected_player {
            prelude::info!("{} was disconnected from lobby", lobby_player.username);
        }
    }
}

#[derive(Debug)]
pub struct StartGame {
    
}

impl prelude::EntityCommand for StartGame {
    type Out = prelude::Result;

    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let host_lobby = entity.get_components_mut::<&mut HostLobby>()
            .expect("Expected HostLobby");
        if host_lobby.started {
            prelude::error!("Game has already started!");
            return Ok(());
        }
        
        entity.reborrow_scope(|entity| {
            let spru_command = spru_bevy::server::command::ManualTrigger::<crate::Server>::new(crate::reaction::Trigger::StartGame);
            spru_command.apply(entity)
        })?;

        entity.resource_scope::<prelude::NextState<crate::AppState>, _>(|_, mut next_state| {
            next_state.set_if_neq(crate::AppState::InGame);
        });

        let mut host_lobby = entity.get_components_mut::<&mut HostLobby>()
            .expect("Expected HostLobby");
        host_lobby.started = true;

        Ok(())
    }
}