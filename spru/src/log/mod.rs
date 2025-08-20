pub(crate) mod client;
use core::fmt;

pub(crate) use client::Client;
pub(crate) mod server;
pub(crate) use server::Server;

use crate::{record, transaction};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, ActionError> {
    #[error(transparent)]
    Record(#[from] record::Error<LookupError, ActionError>),
    #[error(transparent)]
    Revert(#[from] RevertError<LookupError, ActionError>),
}
#[derive(Debug)]
#[derive(thiserror::Error)]
pub struct RevertError<LookupError, ActionError> {
    pub initial: Option<record::Error<LookupError, ActionError>>, 
    pub fatal: record::Error<LookupError, ActionError>,
}

impl<LookupError, ActionError> fmt::Display for RevertError<LookupError, ActionError>
where 
    record::Error<LookupError, ActionError>: fmt::Display
{
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
pub(crate) enum UndoError<LookupError, ActionError> {
    Log(#[from] Error<LookupError, ActionError>),
    Invalid(#[from] transaction::id::InvalidError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ConfirmError<LookupError, ActionError> {
    Log(#[from] Error<LookupError, ActionError>),
    Mismatch(#[from] transaction::id::MismatchError),
}

