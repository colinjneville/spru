pub mod draw;
pub use draw::Draw;
pub mod discard;
pub use discard::Discard;
pub mod play;
pub use play::Play;
use spru::item::IdT;
use tagset::tagset;

pub(crate) type Interactor<'l, 'r, Lookup> = spru::interaction::Interactor<
    'l,
    'r,
    Lookup,
    crate::Actions,
    IdT<crate::game::Root>,
    crate::reaction::Trigger,
>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Out of turn")]
    OutOfTurn,
    #[error("Invalid state")]
    InvalidState,
}

#[tagset(derive(Debug, Clone))]
#[tagset(impl spru::Interaction {
    type State = crate::State;
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
