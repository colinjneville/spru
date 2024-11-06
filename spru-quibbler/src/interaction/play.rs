use crate::{data::Card, interaction};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Play {
    words: Vec<Vec<Card>>,
}

// impl spru::interaction::Base for Play {
//     type Error = interaction::Error;
// }

// impl spru::Interaction<Impl> for Play {
//     fn to_records(&self, interactor: &mut spru::interaction::Interactor<Impl>, player_id: spru::player::Id) -> Result<(), spru::error::LookupInteractionError<Impl, Self::Error>> {
//         todo!()
//     }
// }