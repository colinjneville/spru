use derive_where::derive_where;

use crate::{game, item, player, transaction::Transactions};

#[derive_where(Debug; Common::Root, Common::Action, <Common::State as tagset::TagSet>::Repr)]
#[derive_where(Serialize, Deserialize; Common::Root, Common::Action, <Common::State as tagset::TagSet>::Repr)]
pub struct Seed<Common: super::Common<State: tagset::TagSet>> {
    pub(crate) game_id: game::Id,
    pub(crate) local_player_id: player::Id,
    pub(crate) snapshot: super::Snapshot<<Common::State as tagset::TagSet>::Repr, Common::State, Common::Root>,
    pub(crate) transactions: Transactions<Common::Action>,
    pub(crate) reservation: item::id::Reservation,
}

impl<Common: super::Common<State: tagset::TagSet>> Seed<Common> {
    pub fn game_id(&self) -> game::Id {
        self.game_id
    }

    pub fn local_player_id(&self) -> player::Id {
        self.local_player_id
    }
}
