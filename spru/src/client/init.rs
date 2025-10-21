use derive_where::derive_where;

use crate::{game, item, player, transaction::Transactions, common};

#[derive(Debug)]
#[derive_where(Serialize, Deserialize; <Common as crate::Common>::Root, <Common as crate::Common>::Action)]
pub struct Arg<Common: crate::Common> {
    pub(crate) game_id: game::Id,
    pub(crate) local_player_id: player::Id,
    pub(crate) snapshot: common::Snapshot<Common::State, Common::Root>,
    pub(crate) transactions: Transactions<Common::Action>,
    pub(crate) reservation: item::id::Reservation,
    
}

impl<Common: crate::Common> Arg<Common> {
    pub fn game_id(&self) -> game::Id {
        self.game_id
    }
    
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