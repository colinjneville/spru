use std::time;

use bevy::prelude;

pub(crate) struct RemoteServer;

impl RemoteServer {
    // fn start_listener(
    //     server_add: prelude::On<prelude::Add, spru_bevy::server::component::Runner<crate::Server>>,
    //     mut commands: prelude::Commands,

    // ) -> prelude::Result {
    //     commands
    //         .entity(server_add.entity)
    //         .queue(spru_bevy::server::remote::command::StartListener::<crate::Server>::new(Self::localhost_config()));

    //     Ok(())
    // }

    // fn localhost_config() -> aeronet_webtransport::server::ServerConfig {
    //     let identity = aeronet_webtransport::wtransport::Identity::self_signed(["localhost", "127.0.0.1", "::1"])
    //         .unwrap();
    //     let cert = &identity.certificate_chain().as_slice()[0];
    //     let spki_fingerprint = aeronet_webtransport::cert::spki_fingerprint_b64(cert)
    //         .unwrap();
    //     let cert_hash = aeronet_webtransport::cert::hash_to_b64(cert.hash());
    //     prelude::info!("Generated localhost server config: \n{spki_fingerprint}\n{cert_hash}");

    //     aeronet_webtransport::wtransport::ServerConfig::builder()
    //         .with_bind_default(25576)
    //         .with_identity(identity)
    //         .keep_alive_interval(Some(time::Duration::from_secs(5)))
    //         .max_idle_timeout(Some(time::Duration::from_secs(120)))
    //         .expect("Max timeout should be before the heat death of the universe")
    //         .build()
    // }

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