pub mod draw;
pub use draw::Draw;
pub mod pass;
pub use pass::Pass;
pub mod play;
pub use play::Play;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Out of turn")]
    OutOfTurn,
    #[error("Invalid state")]
    InvalidState
}


#[derive(serde::Serialize, serde::Deserialize)]
pub enum Interaction {
    Draw(Draw),
    Play(Play),
    Pass(Pass),
}

// impl spru::interaction::Base for Interaction {
//     type Error = Error;
// }

// // TEMP
// impl spru::Interaction<Impl> for Interaction {
//     fn to_records(&self, interactor: &mut spru::interaction::Interactor<Impl>, player_id: spru::player::Id) -> Result<(), spru::error::LookupInteractionError<Impl, Self::Error>> {
//         match self {
//             Interaction::Draw(draw) => draw.to_records(interactor, player_id),
//             Interaction::Play(play) => play.to_records(interactor, player_id),
//             Interaction::Pass(pass) => pass.to_records(interactor, player_id),
//         }
//     }
// }