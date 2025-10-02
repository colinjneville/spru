use crate::{item, log, player};

// Not sure this has any value separated
#[derive(Debug)]
pub(crate) struct Core<Action, Root, Interaction> {
    pub(crate) log: log::Client<Action, Interaction>,
    pub(crate) root: Root,
    pub(crate) reservation: item::id::Reservation,
    pub(crate) local_player_id: player::Id,
}