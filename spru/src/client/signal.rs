use crate::{action, transaction};

#[derive(Debug, thiserror::Error)]
#[error("Could not confirm transaction: {0}")]
pub enum ConfirmError {
    Record(#[from] action::Error),
    Mismatch(#[from] transaction::id::MismatchError),
}
