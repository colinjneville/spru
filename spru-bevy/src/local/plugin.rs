use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug, Default)]
pub struct Plugin<Server: crate::server::ServerSSS, Client: crate::client::ClientSSS> {
    _server: PhantomData<fn() -> Server>,
    _client: PhantomData<fn() -> Client>,
}

impl<Server, Client> prelude::Plugin for Plugin<Server, Client> 
where
    Server: crate::server::ServerSSS,
    Client: crate::client::ClientSSS<
        State = Server::State,
        Action = Server::Action,
        Root = Server::Root,
        Interaction = Server::Interaction,
        GameOutcome = <Server::Reaction as spru::Reaction>::GameOutcome,
        Common = Server::Common,
    >,
{
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(prelude::FixedUpdate, (
            super::system::propagate_local_queues::<Server, Client>,
            super::system::create_local_clients::<Server, Client>,
        ));
    }
}