use std::time;

use bevy::prelude;

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
}

impl prelude::Plugin for RemoteServer {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_plugins(spru_bevy::server::remote::Plugin::<crate::Server>::default())
            // .add_observer(
            //     |
            //         mut attempted_connection: prelude::On<spru_bevy::server::remote::event::AttemptedConnection<crate::player::Input>>,
            //     | {
            //         let response;
            //         if let Some(password) = attempted_connection.headers.get("password") 
            //             && let Some(username) = attempted_connection.headers.get("username") 
            //             && password == "password"
            //         {
            //             let input = crate::player::Input::new(username.clone());
                        
            //             response = spru_bevy::server::remote::JoinRequestResponse::AcceptNew(input);
            //         } else {
            //             response = spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed;
            //         }

            //         attempted_connection.set_response(response);
            //     }
            // )
            // .add_observer(Self::start_listener)
            .add_systems(prelude::Startup, (
                Self::skip_main_menu,
            ))
        ;
    }
}