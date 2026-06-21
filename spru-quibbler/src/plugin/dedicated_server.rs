use bevy::prelude;

pub(crate) struct DedicatedServer;

impl prelude::Plugin for DedicatedServer {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_observer(
                |
                    mut a: prelude::On<spru_bevy::server::remote::event::AttemptedConnection<crate::player::Input>>,
                | {
                    let response;
                    if let Some(password) = a.headers.get("password") 
                        && let Some(username) = a.headers.get("username") 
                        && password == "password"
                    {
                        let input = crate::player::Input::new(username.clone());
                        
                        response = spru_bevy::server::remote::JoinRequestResponse::AcceptNew(input);
                    } else {
                        response = spru_bevy::server::remote::JoinRequestResponse::RejectNotAllowed;
                    }

                    a.set_response(response);
                }
            )
        ;
    }
}