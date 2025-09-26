use bevy::prelude;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(prelude::Component)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameId {
    uuid: uuid::Uuid,
}

impl Default for GameId {
    fn default() -> Self {
        Self { 
            uuid: uuid::Uuid::new_v4(),
        }
    }
}