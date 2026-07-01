use std::marker::PhantomData;

use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug, Default; )]
#[derive(prelude::Component, prelude::Reflect)]
pub struct PendingClient<Client>(PhantomData<Client>);

