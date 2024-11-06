use crate::{client, player};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<PlayerInitIn> {
    pub init_input: PlayerInitIn,
}

#[must_use]
pub struct Ret<ItemCatalog, ActionCatalog, Root> {
    pub client_init: client::init::Arg<ItemCatalog, ActionCatalog, Root>,
    pub player_id: player::Id,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}