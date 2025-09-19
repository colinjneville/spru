use std::fmt;

use crate::{action, game, interaction, player, reaction, record, transaction, AnyError};

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Transaction undo failed: {0}")]
pub(crate) enum UndoError {
    Record(action::Error),
    Invalid(#[from] transaction::id::InvalidError),
}

impl From<action::Error> for UndoError {
    fn from(value: action::Error) -> Self {
        Self::Record(value)
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Could not confirm transaction: {0}")]
pub enum ConfirmError {
    Record(action::Error),
    Mismatch(#[from] transaction::id::MismatchError),
}

impl From<action::Error> for ConfirmError {
    fn from(value: action::Error) -> Self {
        Self::Record(value)
    }
}
