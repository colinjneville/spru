use crate::{data::Card, interaction};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Pass {
    discard: Card,
}

// impl spru::interaction::Base for Pass {
//     type Error = interaction::Error;
// }

// impl spru::Interaction<Impl> for Pass {
//     fn to_records(&self, interactor: &mut spru::interaction::Interactor<Impl>, player_id: spru::player::Id) -> Result<(), spru::error::LookupInteractionError<Impl, Self::Error>> {
//         // interactor.param()
//         unimplemented!()
//     }
// }