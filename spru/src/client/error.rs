use std::fmt;

use crate::{action, common::{self, error::FatalError}, interaction, transaction};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InitError {
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum StageInteractionError {
    #[error("{0}")]
    Interaction(#[from] interaction::Error),
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum RevertInteractionError {
    #[error("{0}")]
    InvalidPendingTransaction(#[from] InvalidPendingTransactionError),
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ApplyInteractionError {
    #[error("{0}")]
    InvalidPendingTransaction(#[from] InvalidPendingTransactionError),
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum SignalError {
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("The pending transaction {transaction} does not exist, or has already been applied or reverted")]
pub struct InvalidPendingTransactionError {
    pub transaction: interaction::Pending,
}

impl InvalidPendingTransactionError {
    pub(crate) fn new(transaction: interaction::Pending) -> Self {
        Self {
            transaction,
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum TransactionConfirmationError {
    #[error("{0}")]
    Action(#[from] action::Error),
    #[error("{0}")]
    Mismatch(#[from] transaction::id::MismatchError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub(crate) enum TransactionOutOfOrderError {
    #[error("Expected pending id {}, received {actual}", expected.as_ref().map(|tp| tp as &dyn fmt::Display).unwrap_or(&"None"))]
    WrongPendingId { expected: Option<interaction::Pending>, actual: interaction::Pending },
    #[error("Expected confirmed id {expected}, received {actual}")]
    WrongConfirmdId { expected: transaction::Id, actual: transaction::Id },
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub(crate) enum ConfirmPendingError {
    #[error("{0}")]
    Action(#[from] common::error::RecoverableError<action::Error>),
    #[error("{0}")]
    OutOfOrder(#[from] TransactionOutOfOrderError),
}
