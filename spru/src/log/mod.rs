pub(crate) mod client;
use core::fmt;

pub(crate) use client::Client;
pub(crate) mod server;
pub(crate) use server::Server;

use crate::{record, transaction, CustomError};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Record(record::Error),
    #[error(transparent)]
    Revert(#[from] RevertError),
}

impl From<record::Error> for Error {
    fn from(value: record::Error) -> Self {
        Self::Record(value)
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
#[non_exhaustive]
pub struct RevertError {
    pub initial: Option<CustomError>, 
    pub fatal: record::Error,
}

impl RevertError {
    pub(crate) fn new<E: (initial: )
}

impl fmt::Display for RevertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            initial,
            fatal,
        } = self;

        write!(f, "An error occurred while applying a record")?;
        if let Some(initial) = initial {
            write!(f, " ({initial})")?;
        }
        write!(f, ", but could not be reverted: {fatal}")
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Transaction undo failed: {0}")]
pub(crate) enum UndoError {
    Log(#[from] Error),
    Invalid(#[from] transaction::id::InvalidError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Could not confirm transaction: {0}")]
pub enum ConfirmError {
    Log(#[from] Error),
    Mismatch(#[from] transaction::id::MismatchError),
}

