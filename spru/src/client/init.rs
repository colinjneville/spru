use crate::{item, player, transaction::Transactions, Snapshot};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<ItemCatalog, ActionCatalog, Root> {
    pub(crate) snapshot: Snapshot<ItemCatalog, Root>,
    pub(crate) transactions: Transactions<ActionCatalog>,
    pub(crate) reservation: item::id::Reservation,
    pub(crate) local_player_id: player::Id,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}