use std::marker::PhantomData;

use bevy::{ecs::system::IntoSystem as _, prelude};
use derive_where::derive_where;

#[derive_where(Debug, Default; )]
pub struct Plugin<Server: crate::server::ServerSSS> {
    _server: PhantomData<fn() -> Server>,
}

impl<Server: crate::server::ServerSSS> prelude::Plugin for Plugin<Server> {
    fn build(&self, app: &mut prelude::App) {
        let Self { _server } = self;

        app.add_systems(
            prelude::FixedUpdate,
            (super::system::run_server::<Server>
                .pipe::<_, _, prelude::Result, _>(crate::common::adapt::map_err),),
        );
    }
}
