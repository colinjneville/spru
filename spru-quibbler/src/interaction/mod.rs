pub mod draw;
pub use draw::Draw;
pub mod discard;
pub use discard::Discard;
pub mod play;
pub use play::Play;
use spru::item::IdT;
use tagset::tagset;

pub(crate) type Interactor<'l, 'r, Lookup> = spru::interaction::Interactor<'l, 'r, Lookup, crate::Actions, IdT<crate::game::Root>, crate::reaction::Trigger>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Out of turn")]
    OutOfTurn,
    #[error("Invalid state")]
    InvalidState
}

#[tagset(impl spru::Interaction {
    type Action = crate::Actions; 
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(Draw)]
#[tagset(Play)]
#[tagset(Discard)]
pub struct Interaction;

pub struct Output;

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