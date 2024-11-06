use crate::{item, player, transaction, Snapshot};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Save<ItemCatalog, Root, PlayerInit> {
    pub(crate) snapshot: Snapshot<ItemCatalog, Root>,
    pub(crate) next_transaction_id: transaction::Id,
    pub(crate) reservation: item::id::Range,
    pub(crate) player_manager: player::Manager<PlayerInit>,
}
