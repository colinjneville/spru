use std::collections;

use bevy::prelude;

#[derive(Debug)]
#[derive(prelude::EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct AttemptedConnection<PlayerInitIn> {
    // Triggers on the newly created remote client entity, which is a child of the server.
    pub entity: prelude::Entity,
    pub headers: collections::HashMap<String, String>,
    pub response: Option<super::JoinRequestResponse<PlayerInitIn>>,
}

impl<PlayerInitIn> AttemptedConnection<PlayerInitIn> {
    pub fn set_response(&mut self, response: super::JoinRequestResponse<PlayerInitIn>) {
        self.response = Some(response);
    }
}