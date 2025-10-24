use derive_where::derive_where;

use crate::{game, item, player, transaction::Transactions};

#[derive(Debug)]
#[derive_where(Serialize, Deserialize; <Common as super::Common>::Root, <Common as super::Common>::Action)]
pub struct Seed<Common: super::Common> {
    pub(crate) game_id: game::Id,
    pub(crate) local_player_id: player::Id,
    pub(crate) snapshot: super::Snapshot<Common::State, Common::Root>,
    pub(crate) transactions: Transactions<Common::Action>,
    pub(crate) reservation: item::id::Reservation,
    
}

impl<Common: super::Common> Seed<Common> {
    pub fn game_id(&self) -> game::Id {
        self.game_id
    }
    
    pub fn local_player_id(&self) -> player::Id {
        self.local_player_id
    }
}