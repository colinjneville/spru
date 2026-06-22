use std::marker::PhantomData;

use bevy::{ecs::system::IntoSystem as _, prelude};
use derive_where::derive_where;

use crate::client;

#[derive_where(Debug, Default)]
pub struct Plugin<Client> {
    _client: PhantomData<fn() -> Client>,
}

impl<Client: super::ClientSSS> prelude::Plugin for Plugin<Client> {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_systems(
                prelude::FixedPostUpdate,
                (super::system::run_client::<Client>
                    .pipe::<_, _, prelude::Result, _>(crate::common::adapt::map_err),),
            )
            .init_resource::<client::resource::ClientMap>()
        ;
    }
}
