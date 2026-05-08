use spru::player;

#[derive(Debug, Clone, thiserror::Error)]
#[error("Player with id {player_id} already exists")]
pub struct PlayerAlreadyExists {
    pub player_id: player::Id,
}

impl PlayerAlreadyExists {
    pub(crate) fn new(player_id: player::Id) -> Self {
        Self {
            player_id,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Player with id {player_id} does not exist")]
pub struct PlayerDoesNotExist {
    pub player_id: player::Id,
}

impl PlayerDoesNotExist {
    pub(crate) fn new(player_id: player::Id) -> Self {
        Self {
            player_id,
        }
    }
}