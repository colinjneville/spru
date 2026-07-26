
use bevy::prelude;

pub struct StartJoinLobby {
    
}

impl prelude::EntityCommand for StartJoinLobby {
    type Out = ();

    fn apply(self, mut entity: bevy::ecs::world::EntityWorldMut) -> Self::Out {
        let Self {
            
        } = self;

        entity
            .insert(super::Connecting { })
            .observe(JoinLobby::on_connected)
            .observe(JoinLobby::on_disconnected)
            .world_scope(|world| {
                prelude::debug!("transitioning to {:?}", crate::AppState::Connecting);
                world.insert_resource(prelude::NextState::PendingIfNeq(crate::AppState::Connecting));
            })
            ;
    }
}

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct JoinLobby {

}

impl JoinLobby {
    

    // This should fire whether the connection request was rejected, or we were disconnected after 
    // a successful request.
    fn on_disconnected(
        disconnected: prelude::On<spru_bevy::client::remote::event::Disconnected>,
        mut commands: prelude::Commands,
        mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
        q_join_lobby: prelude::Query<(
            Option<&spru_bevy::client::remote::component::ConnectionConfig>,
        )>,
    ) {
        let _span = prelude::error_span!("JoinLobby::on_disconnected").entered();
        
        let entity = disconnected.entity;
        let (connection_config, ) = q_join_lobby.get(entity).unwrap();

        // If the disconnection was due to an error, and our connection request succeeded, 
        // if we have connection info, attempt to reconnect.
        // Otherwise, return to menu.
        if let Some(connection_config) = connection_config && let Some(err) = disconnected.reason.by_error() {
            prelude::info!("Client disconnected, attempting reconnection: {err}");

            commands
                .entity(entity)
                .queue(spru_bevy::client::command::Shutdown::<crate::Client>::new(false))
                .queue(spru_bevy::client::remote::command::JoinRemote::<crate::Client>::new(connection_config.clone()))
                ;
        } else {
            prelude::info!("Client disconnected, returning to main menu");

            commands
                .entity(entity)
                .queue(spru_bevy::client::command::Shutdown::<crate::Client>::new(true));

            next_state.set_if_neq(crate::AppState::MainMenu);
        }
    }
}