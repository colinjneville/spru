use std::marker::PhantomData;

use derive_where::derive_where;

use crate::interaction;

#[derive_where(Debug; GameComplete<Client>)]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum Event<Client: super::Client> {
    InteractionResult(InteractionResult<Client>),
    GameComplete(GameComplete<Client>),
}

#[derive_where(Debug; Client::GameOutcome)]
#[non_exhaustive]
pub struct GameComplete<Client: super::Client> {
    pub game_outcome: Client::GameOutcome,
}

#[derive_where(Debug; )]
#[non_exhaustive]
pub struct InteractionResult<Client: super::Client> {
    pub pending_interaction_id: interaction::Pending,
    pub confirmed: bool,
    pub(crate) _client: PhantomData<fn() -> Client>,
}
