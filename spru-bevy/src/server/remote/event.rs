use std::collections;

use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
pub struct AttemptedConnection<PlayerInitIn> {
    pub entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub response: Option<super::JoinRequestResponse<PlayerInitIn>>,
}

impl<PlayerInitIn> AttemptedConnection<PlayerInitIn> {
    pub fn set_response(&mut self, response: super::JoinRequestResponse<PlayerInitIn>) {
        self.response = Some(response);
    }
}