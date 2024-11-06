use spru_bevy::item::{Id, IdT};
use spru_util::{Strictness, item::{pile, fsm}};

use crate::interaction;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum DrawLocation {
    Deck,
    Discard,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Deck,
    Discard,
}

// impl spru::interaction::Base for Draw {
//     type Error = interaction::Error;
// }

// impl spru::Interaction<Impl> for Draw {
//     fn to_records(&self, interactor: &mut Interactor<Impl>, player_id: player::Id) -> Result<(), LookupInteractionError<Impl, Self::Error>> {
//         let root = interactor.param();
//         let root = interactor.lookup().lookup(&root).map_err(LookupInteractionError::Lookup)?;

//         let player_root = interactor.player_manager()[player_id].root;
//         let player_root = interactor.lookup().lookup(&player_root).map_err(LookupInteractionError::Lookup)?;

//         interactor.modify(&player_root.state, fsm::update::Transition::new(crate::player::StateInput::Draw)).map_err(LookupInteractionError::Lookup)?;

//         let pile_id = match self {
//             Draw::Deck => &root.deck,
//             Draw::Discard => &root.discard,
//         }.clone();

//         let pile = interactor.lookup().lookup(&pile_id).map_err(LookupInteractionError::Lookup)?;
//         let card = pile.top().ok_or(interaction::Error::InvalidState).map_err(LookupInteractionError::Interaction)?;
//         interactor.modify(&pile_id, pile::modify::PopTop::new(Strictness::AllOrError)).map_err(LookupInteractionError::Lookup)?;
        
//         // interactor.player_manager()[player_id].init
//         unimplemented!()
//     }
// }