use std::marker::PhantomData;

use bevy::{ecs::system::IntoSystem as _, prelude};
use derive_where::derive_where;

use crate::{common, server};

#[derive_where(Debug, Default; )]
pub struct Plugin<Server: server::ServerSSS> {
    _p: PhantomData<Server>,
}

impl<Server: server::ServerSSS> prelude::Plugin for Plugin<Server> {
    fn build(&self, app: &mut prelude::App) {
        let Self { _p } = self;

        app
            .add_systems(
                prelude::FixedUpdate,
                (server::system::run_server::<Server>
                    .pipe::<_, _, prelude::Result, _>(common::adapt::map_err),),
            )
            .init_resource::<server::resource::ServerMap>()
        ;
    }
}
