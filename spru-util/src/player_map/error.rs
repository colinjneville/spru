use spru::player;

#[derive(Debug, Clone)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Player with id {0} already exists")]
    PlayerAlreadyExists(player::Id),
    #[error("Player with id {0} does not exist")]
    PlayerDoesNotExist(player::Id),
}