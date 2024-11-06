pub(crate) mod client;
use core::fmt;

pub(crate) use client::Client;
pub(crate) mod server;
pub(crate) use server::Server;

use crate::{record, transaction};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error<LookupError, ActionCatalogError> {
    #[error(transparent)]
    Record(#[from] record::Error<LookupError, ActionCatalogError>),
    #[error(transparent)]
    Revert(#[from] RevertError<LookupError, ActionCatalogError>),
}
#[derive(Debug)]
#[derive(thiserror::Error)]
pub struct RevertError<LookupError, ActionCatalogError> {
    pub initial: Option<record::Error<LookupError, ActionCatalogError>>, 
    pub fatal: record::Error<LookupError, ActionCatalogError>,
}

impl<LookupError, ActionCatalogError> fmt::Display for RevertError<LookupError, ActionCatalogError>
where 
    record::Error<LookupError, ActionCatalogError>: fmt::Display
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
pub(crate) enum UndoError<LookupError, ActionCatalogError> {
    Log(#[from] Error<LookupError, ActionCatalogError>),
    Invalid(#[from] transaction::id::InvalidError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ConfirmError<LookupError, ActionCatalogError> {
    Log(#[from] Error<LookupError, ActionCatalogError>),
    Mismatch(#[from] transaction::id::MismatchError),
}

