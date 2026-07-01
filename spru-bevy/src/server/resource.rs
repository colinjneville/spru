use std::collections::HashMap;

use bevy::prelude;

#[derive(Debug, Default)]
#[derive(prelude::Resource, prelude::Reflect)]
pub struct ServerMap {
    map: HashMap<crate::reflect::spru::game::Id, prelude::Entity>,
    inverse_map: HashMap<prelude::Entity, crate::reflect::spru::game::Id>,
}

impl ServerMap {
    pub(crate) fn insert(&mut self, game_id: spru::game::Id, entity: prelude::Entity) {
        self.map.insert(crate::reflect::spru::game::Id(game_id), entity);
        self.inverse_map.insert(entity, crate::reflect::spru::game::Id(game_id));
    }

    pub(crate) fn remove(&mut self, entity: prelude::Entity) {
        if let Some(game_id) = self.inverse_map.remove(&entity) {
            self.map.remove(&game_id);
        }
    }

    pub fn get(&self, game_id: spru::game::Id) -> Option<prelude::Entity> {
        self.map.get(&crate::reflect::spru::game::Id(game_id)).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (spru::game::Id, prelude::Entity)> {
        self.map.iter().map(|(id, &entity)| (id.0, entity))
    }
}
