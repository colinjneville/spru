use crate::{item, player, transaction, Snapshot};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Save<State, Root, PlayerInit, Reaction> {
    pub(crate) snapshot: Snapshot<State, Root>,
    pub(crate) next_transaction_id: transaction::Id,
    pub(crate) reservation: item::id::Range,
    pub(crate) player_manager: player::Manager<PlayerInit>,
    pub(crate) reaction: Reaction,
}
