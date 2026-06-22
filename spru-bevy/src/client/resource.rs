use std::collections::HashMap;

use bevy::prelude;

#[derive(Debug, Default)]
#[derive(prelude::Resource)]
pub struct ClientMap {
    map: HashMap<(spru::game::Id, spru::player::Id), prelude::Entity>,
    inverse_map: HashMap<prelude::Entity, (spru::game::Id, spru::player::Id)>,
}

impl ClientMap {
    pub(crate) fn insert(&mut self, game_id: spru::game::Id, player_id: spru::player::Id, entity: prelude::Entity) {
        self.map.insert((game_id, player_id), entity);
        self.inverse_map.insert(entity, (game_id, player_id));
    }

    pub(crate) fn remove(&mut self, entity: prelude::Entity) {
        if let Some(ids) = self.inverse_map.remove(&entity) {
            self.map.remove(&ids);
        }
    }

    pub fn get(&self, game_id: spru::game::Id, player_id: spru::player::Id) -> Option<prelude::Entity> {
        self.map.get(&(game_id, player_id)).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (spru::game::Id, spru::player::Id, prelude::Entity)> {
        self.map.iter()
            .map(|(&(game_id, player_id), &entity)| (game_id, player_id, entity))
    }
}
