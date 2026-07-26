use std::collections::HashMap;

use bevy::prelude;

#[derive(Default)]
#[derive(prelude::Resource)]
pub(crate) struct ActiveClient(HashMap<spru::game::Id, spru::player::Id>);

impl ActiveClient {
    pub fn get(&self, active_game: spru::game::Id) -> Option<spru::player::Id> {
        self.0.get(&active_game).copied()
    }

    pub fn set(&mut self, active_game: spru::game::Id, value: Option<spru::player::Id>) {
        if let Some(value) = value {
            prelude::trace!("Setting ActiveClient for {active_game} to {value}");
            self.0.insert(active_game, value);
        } else {
            prelude::trace!("Setting ActiveClient for {active_game} to None");
            self.0.remove(&active_game);
        }
    }
}