use bevy::prelude;


#[derive(prelude::Component)]
pub struct Connecting {}

#[derive(prelude::Component)]
pub(super) struct ToLobby;

impl Connecting {
    fn on_connected(
        connected: prelude::On<spru_bevy::client::remote::event::Connected>,
        mut commands: prelude::Commands,
        mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
        q_connecting: prelude::Query<(
            &Connecting,
            Option<&ToLobby>,
        )>,
    ) {
        if let Ok((connecting, to_lobby, )) = q_connecting.get(connected.entity) {
            prelude::debug!("Connection complete, transitioning to lobby");
            let mut entity_commands = commands.entity(connected.entity);

            entity_commands
                .remove::<Connecting>();

            if let Some(_to_lobby) = to_lobby {
                entity_commands
                    .remove::<ToLobby>()
                    .insert(super::JoinLobby { })
                    ;

                next_state.set_if_neq(crate::AppState::InLobby);
            }
        }
    }

    pub(super) fn on_disconnected(
        disconnected: prelude::On<spru_bevy::client::remote::event::Disconnected>,
        mut commands: prelude::Commands,
        q_connecting: prelude::Query<(
            &Connecting,
            Option<&ToLobby>,
        )>,
    ) {
        if let Ok((_connecting, to_lobby, )) = q_connecting.get(disconnected.entity) {
            if let Some(_to_lobby) = to_lobby {
                prelude::info!("Client disconnected, returning to main menu");

                commands.entity(disconnected.entity)
                    .remove::<Connecting>()
                    ;
            } else {
                
            }
            
        }
    }
}