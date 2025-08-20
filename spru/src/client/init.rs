use crate::{item, player, transaction::Transactions, Snapshot};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<State, Action, Root> {
    pub(crate) snapshot: Snapshot<State, Root>,
    pub(crate) transactions: Transactions<Action>,
    pub(crate) reservation: item::id::Reservation,
    pub(crate) local_player_id: player::Id,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}