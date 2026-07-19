use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

use crate::client;

#[derive_where(Debug, Default)]
pub struct Plugin<Client> {
    _client: PhantomData<Client>,
}

impl<Client: super::ClientSSS> prelude::Plugin for Plugin<Client> {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_systems(
                prelude::FixedPostUpdate,
                super::system::run_client::<Client>,
            )
            .init_resource::<client::resource::ClientMap>()
        ;
    }
}
