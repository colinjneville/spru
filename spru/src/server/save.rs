use derive_where::derive_where;

use crate::{common, game, item, player, transaction};

#[derive_where(Debug, Serialize, Deserialize; 
    common::Snapshot<Server::State, Server::Root>, 
    player::Manager<Server::PlayerInit>,
    Server::Reaction,
)]
pub struct Save<Server: super::Server> {
    pub(crate) game_id: game::Id,
    pub(crate) snapshot: common::Snapshot<Server::State, Server::Root>,
    pub(crate) next_transaction_id: transaction::Id,
    pub(crate) reservation: item::id::Range,
    pub(crate) player_manager: player::Manager<Server::PlayerInit>,
    pub(crate) reaction: Server::Reaction,
}

pub type Result<Server> = std::result::Result<Save<Server>, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Snapshot(#[from] common::error::Save),
}

