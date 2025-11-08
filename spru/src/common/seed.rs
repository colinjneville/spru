use derive_where::derive_where;

use crate::{game, item, player, transaction::Transactions};

/// Used to create a new [Client](crate::Client) when a player is added.
/// Call [Client::init](crate::Client::init) to create the client.
#[derive_where(Debug; Common::Root, Common::Action, <Common::State as tagset::TagSet>::Repr)]
#[derive_where(Serialize, Deserialize; Common::Root, Common::Action, <Common::State as tagset::TagSet>::Repr)]
pub struct Seed<Common: super::Common<State: tagset::TagSet>> {
    pub(crate) game_id: game::Id,
    pub(crate) local_player_id: player::Id,
    pub(crate) snapshot:
        super::Snapshot<<Common::State as tagset::TagSet>::Repr, Common::State, Common::Root>,
    pub(crate) transactions: Transactions<Common::Action>,
    pub(crate) reservation: item::id::Reservation,
    // TODO spru versioning
}

impl<Common: super::Common<State: tagset::TagSet>> Seed<Common> {
    /// The [game::Id] this is for
    pub fn game_id(&self) -> game::Id {
        self.game_id
    }

    /// The [player::Id] the [Client](crate::Client) will be created for
    pub fn local_player_id(&self) -> player::Id {
        self.local_player_id
    }
}
