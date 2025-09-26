use crate::{item, player, transaction::Transactions, Snapshot};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<State, Action, Root> {
    pub(crate) snapshot: Snapshot<State, Root>,
    pub(crate) transactions: Transactions<Action>,
    pub(crate) reservation: item::id::Reservation,
    pub(crate) local_player_id: player::Id,
}

impl<State, Action, Root> Arg<State, Action, Root> {
    pub fn local_player_id(&self) -> player::Id {
        self.local_player_id
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}