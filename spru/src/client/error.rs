//! Errors returned by [Client](crate::Client) operations

use std::fmt;

use crate::{
    action,
    common::{self, error::FatalError},
    interaction, transaction,
};

/// An error occurred during [Client::init](crate::Client::init)
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// Fatal error, the client must be recreated from a [Seed](common::Seed)
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(InitError);

/// An error occurred during [Client::stage_interaction](crate::Client::stage_interaction)
#[derive(Debug, thiserror::Error)]
pub enum StageInteractionError {
    /// [Interaction](trait@crate::Interaction) error
    #[error("{0}")]
    Interaction(#[from] interaction::Error),
    /// Fatal error, the client must be recreated from a [Seed](common::Seed)
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(StageInteractionError);

/// An error occurred during [Client::revert_interactions](crate::Client::revert_interactions)
#[derive(Debug, thiserror::Error)]
pub enum RevertInteractionError {
    /// The [interaction::Pending] is invalid
    #[error("{0}")]
    InvalidInteractionPending(#[from] InvalidInteractionPendingError),
    /// Fatal error, the client must be recreated from a [Seed](common::Seed)
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(RevertInteractionError);

/// An error occurred during [Client::apply_interactions](crate::Client::apply_interactions)
#[derive(Debug, thiserror::Error)]
pub enum ApplyInteractionError {
    /// The [interaction::Pending] is invalid
    #[error("{0}")]
    InvalidInteractionPending(#[from] InvalidInteractionPendingError),
    /// Fatal error, the client must be recreated from a [Seed](common::Seed)
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(ApplyInteractionError);

/// An error occurred during [Client::signal](crate::Client::signal)
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    /// Fatal error, the client must be recreated from a [Seed](common::Seed)
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(SignalError);

/// An error occurred during [Client::shutdown](crate::Client::shutdown)
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// All errors in `destroy` are fatal, as `destroy` itself is fatal`
    #[error("{0}")]
    Fatal(#[from] FatalError),
}

common::error::impl_get_fatal_error!(ShutdownError);

/// The client has been shutdown by [Client::shutdown](crate::Client::shutdown)
/// and cannot be used further.
#[derive(Debug, thiserror::Error)]
#[error("The client has been shutdown")]
pub struct IsShutdownError;

/// The provided pending interation does not exist, or has already been applied or reverted
#[derive(Debug, thiserror::Error)]
#[error(
    "The pending interaction {pending_interaction} does not exist, or has already been applied or reverted"
)]
pub struct InvalidInteractionPendingError {
    /// The invalid pending interaction
    pub pending_interaction: interaction::Pending,
}

impl InvalidInteractionPendingError {
    pub(crate) fn new(pending_interaction: interaction::Pending) -> Self {
        Self {
            pending_interaction,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TransactionConfirmationError {
    #[error("{0}")]
    Action(#[from] action::Error),
    #[error("{0}")]
    Mismatch(#[from] transaction::id::MismatchError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TransactionOutOfOrderError {
    #[error("Expected pending id {}, received {actual}", expected.as_ref().map(|tp| tp as &dyn fmt::Display).unwrap_or(&"None"))]
    WrongPendingId {
        expected: Option<interaction::Pending>,
        actual: interaction::Pending,
    },
    #[error("Expected confirmed id {expected}, received {actual}")]
    WrongConfirmdId {
        expected: transaction::Id,
        actual: transaction::Id,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfirmPendingError {
    #[error("{0}")]
    Action(#[from] common::error::RecoverableError<action::Error>),
    #[error("{0}")]
    OutOfOrder(#[from] TransactionOutOfOrderError),
}
