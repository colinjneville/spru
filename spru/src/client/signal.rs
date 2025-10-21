use derive_where::derive_where;

use crate::{action, item::lookup, record::Records, transaction, common};

#[derive_where(Debug, Serialize, Deserialize; Internal<Common>)]
pub struct Signal<Common: crate::Common> {
    pub(crate) seq: common::SeqId,
    pub(crate) signal: Internal<Common>,
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
pub enum Error {
    #[error("{0}")]
    Lookup(lookup::Error),
    #[error("{0}")]
    Action(action::Error),
    // TODO
    #[error("Other synchronization error: {0}")]
    Other(crate::TempError),
}

impl From<crate::TempError> for Error {
    fn from(value: crate::TempError) -> Self {
        Self::Other(value)
    }
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::Lookup(value)
    }
}

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::Action(value)
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(derive_more::From)]
#[derive_where(Debug, Serialize, Deserialize; InteractionResult<Common>, ConfirmedTransaction<Common>, EndGame<Common>)]
pub(crate) enum Internal<Common: crate::Common> {
    InteractionResult(InteractionResult<Common>),
    ConfirmedTransaction(ConfirmedTransaction<Common>),
    EndGame(EndGame<Common>),
}

#[derive_where(Debug, Serialize, Deserialize; Records<Common::Action>)]
pub(crate) struct InteractionResult<Common: crate::Common> {
    pub pending_transaction_id: transaction::Pending,
    pub confirmed_transaction_id: Option<(transaction::Id, Records<Common::Action>)>,
}

#[derive_where(Debug, Serialize, Deserialize; transaction::Confirmed<Common::Action>)]
pub(crate) struct ConfirmedTransaction<Common: crate::Common> {
    pub confirmed_transaction: transaction::Confirmed<Common::Action>,
}

#[derive_where(Debug, Serialize, Deserialize; Common::GameOutcome)]
pub(crate) struct EndGame<Common: crate::Common> {
    pub game_outcome: Common::GameOutcome,
}
