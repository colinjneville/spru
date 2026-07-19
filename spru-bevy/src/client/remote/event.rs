use std::fmt;

use bevy::prelude;

use crate::remote;

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Connected {
    #[event_target]
    pub entity: prelude::Entity,
}

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct Disconnected {
    #[event_target]
    pub entity: prelude::Entity,
    pub reason: remote::DisconnectedReason,
}