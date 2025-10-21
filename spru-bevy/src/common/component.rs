use std::fmt;

use bevy::prelude;
use derive_where::derive_where;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(prelude::Component)]
#[component(immutable)]
pub struct GameId(pub spru::game::Id);

impl GameId {
    pub(crate) fn new(game_id: spru::game::Id) -> Self {
        Self(game_id)
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
