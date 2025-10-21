use derive_where::derive_where;

use crate::{error::RecoverableError, interaction, transaction};

#[derive(Debug)]
#[must_use]
pub struct Ret {
    pub pending_transaction_id: transaction::Pending,
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("The interaction could not staged: {0}")]
    Interaction(RecoverableError<interaction::Error>),
}

impl From<RecoverableError<interaction::Error>> for Error {
    fn from(value: RecoverableError<interaction::Error>) -> Self {
        Self::Interaction(value)
    }
}