use crate::{record::Records, transaction};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<Action, GameOutcome> {
    pub(crate) signal: Internal<Action, GameOutcome>,
}

#[derive(Debug)]
#[must_use]
pub struct Ret { }

impl Ret {
    pub(crate) fn new() -> Self {
        Self {

        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, ActionsError> {
    #[error(transparent)]
    Lookup(LookupError),
    #[error(transparent)]
    Action(ActionsError),
    // TODO
    #[error("Other synchronization error: {0}")]
    Other(crate::TempError),
}

impl<LookupError, ActionsError> From<crate::TempError> for Error<LookupError, ActionsError> {
    fn from(value: crate::TempError) -> Self {
        Self::Other(value)
    }
}

#[derive(Debug)]
#[derive(derive_more::From)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum Internal<Action, GameOutcome> {
    InteractionResult(InteractionResult<Action>),
    ConfirmedTransaction(ConfirmedTransaction<Action>),
    EndGame(EndGame<GameOutcome>),
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct InteractionResult<Action> {
    pub pending_transaction_id: transaction::Pending,
    pub confirmed_transaction_id: Option<(transaction::Id, Records<Action>)>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfirmedTransaction<Action> {
    pub confirmed_transaction: transaction::Confirmed<Action>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct EndGame<GameOutcome> {
    pub game_outcome: GameOutcome,
}
