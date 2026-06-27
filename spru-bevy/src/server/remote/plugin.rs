use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug, Default; )]
pub struct Plugin<Server: crate::server::ServerSSS> {
    _server: PhantomData<fn() -> Server>,
}

impl<Server> prelude::Plugin for Plugin<Server>
where
    Server: crate::server::ServerSSS<
        Action: serde::Serialize,
        Reaction: spru::Reaction<GameOutcome: serde::Serialize>,
        Interaction: serde::de::DeserializeOwned,
        Root: serde::Serialize,
        State: spru::State<Repr: serde::Serialize>,
    >,
{
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins(aeronet::AeronetPlugins)
            .add_systems(
                prelude::FixedUpdate,
                (
                    super::system::propagate_remote_queues::<Server>,
                    // super::system::create_local_clients::<Server, Client>,
                ),
            )
            .add_observer(super::observer::on_opened)
            .add_observer(super::observer::on_session_request::<Server>)
            .add_observer(super::observer::on_connected::<Server>)
        ;
    }
}
