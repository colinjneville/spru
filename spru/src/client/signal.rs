use crate::transaction;

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<ActionCatalog, GameOutcome> {
    pub(crate) signal: Internal<ActionCatalog, GameOutcome>,
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
    #[error("Other synchronization error")]
    Other,
}

impl<LookupError, ActionsError> From<crate::TempError> for Error<LookupError, ActionsError> {
    fn from(_value: crate::TempError) -> Self {
        Self::Other
    }
}

#[derive(Debug)]
#[derive(derive_more::From)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum Internal<ActionCatalog, GameOutcome> {
    InteractionResult(InteractionResult),
    ConfirmedTransaction(ConfirmedTransaction<ActionCatalog>),
    EndGame(EndGame<GameOutcome>),
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct InteractionResult {
    pub pending_transaction_id: transaction::Pending,
    pub confirmed_transaction_id: Option<transaction::Id>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ConfirmedTransaction<ActionCatalog> {
    pub confirmed_transaction: transaction::Confirmed<ActionCatalog>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct EndGame<GameOutcome> {
    pub game_outcome: GameOutcome,
}
