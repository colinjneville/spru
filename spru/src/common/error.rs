//! Errors shared between client and server code

use std::{fmt, ops, sync::Arc};

use crate::{action, item::storage, transaction};

/// An error occurred while saving game state
#[derive(Debug, thiserror::Error)]
#[error("An error occurred while saving game state: {0}")]
pub enum Save {
    /// Serializing the game state failed
    Serialization(#[from] rmp_serde::encode::Error),
}

#[doc(hidden)]
#[derive(Debug, thiserror::Error)]
#[error("An error occurred while loading game state: {0}")]
pub enum Load {
    Deserialization(#[from] rmp_serde::decode::Error),
    Storage(storage::Error),
}

impl From<storage::Error> for Load {
    fn from(value: storage::Error) -> Self {
        Self::Storage(value)
    }
}

/// A type-erased error
#[derive(Debug)]
pub struct AnyError {
    inner: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl AnyError {
    /// Create an error from a string
    pub fn from_string<S: Into<String>>(s: S) -> Self {
        let s = s.into();
        Self {
            inner: Box::from(s),
        }
    }

    pub(crate) fn new<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self { inner: Box::new(e) }
    }

    /// Get a reference to the inner dynamic error
    pub fn get(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.inner
    }

    /// Attempt to downcast the error to the given concrete type
    pub fn try_cast<E: std::error::Error + Send + Sync + 'static>(self) -> Result<E, Self> {
        let Self { mut inner } = self;

        match inner.downcast() {
            Ok(e) => return Ok(*e),
            Err(e) => inner = e,
        }

        Err(Self { inner })
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for AnyError {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl ops::Deref for AnyError {
    type Target = dyn std::error::Error;

    fn deref(&self) -> &Self::Target {
        self.as_error()
    }
}

/// A result with an [AnyError] `Err`
pub type AnyResult<T> = std::result::Result<T, AnyError>;

impl PseudoError for AnyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::error::Error::source(self.get())
    }
}

/// An error type which does not implement [std::error::Error] to avoid conflicting [From]
/// implementations, but can be costlessly converted to one when needed.
pub trait PseudoError: fmt::Debug + fmt::Display {
    /// Equivalent to [std::error::Error::source]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>;

    /// Convert this type to a type implementing [std::error::Error]
    fn into_error(self) -> ImplError<Self>
    where
        Self: Sized,
    {
        ImplError::new(self)
    }

    /// Convert this reference to a reference to a type implementing [std::error::Error]
    fn as_error(&self) -> &ImplError<Self> {
        ImplError::new_ref(self)
    }
}

/// A wrapper around a [PsuedoError] which implements [std::error::Error]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImplError<E: ?Sized>(E);

impl<E> ImplError<E> {
    fn new(e: E) -> Self {
        Self(e)
    }
}

impl<E: ?Sized> ImplError<E> {
    fn new_ref(e: &E) -> &Self {
        // SAFETY: transparent struct refs can be transmuted safely
        unsafe { std::mem::transmute(e) }
    }
}

impl<E: PseudoError> fmt::Display for ImplError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<E: PseudoError> std::error::Error for ImplError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        PseudoError::source(&self.0)
    }
}

#[derive(Debug)]
pub(crate) struct RecoverableError<E> {
    pub initial_error: E,
    pub recovery_error: Option<Box<action::Error>>,
}

impl<E: PseudoError + 'static> ops::Deref for RecoverableError<E> {
    type Target = dyn std::error::Error;

    fn deref(&self) -> &Self::Target {
        // SAFETY: This is safe as long as the layout of a containing type (RecoverableError)
        // does not change between identical layout fields (E/ImplError<E>).
        // At the very least, Miri does not complain
        let e: &RecoverableError<ImplError<E>> = unsafe { std::mem::transmute(self) };
        e
    }
}

impl<E> RecoverableError<E> {
    pub(crate) fn new(initial_error: E) -> Self {
        Self {
            initial_error,
            recovery_error: None,
        }
    }

    pub(crate) fn set_recovery_error(&mut self, recovery_error: action::Error) {
        self.recovery_error = Some(Box::new(recovery_error));
    }

    pub fn is_recovered(&self) -> bool {
        self.recovery_error.is_none()
    }

    pub(crate) fn map<E2>(self) -> RecoverableError<E2>
    where
        E: Into<E2>,
    {
        let Self {
            initial_error,
            recovery_error,
        } = self;

        let initial_error: E2 = initial_error.into();

        RecoverableError {
            initial_error,
            recovery_error,
        }
    }

    pub(crate) fn map_with<F, E2>(self, f: F) -> RecoverableError<E2>
    where
        F: FnOnce(E) -> E2,
    {
        let Self {
            initial_error,
            recovery_error,
        } = self;

        let initial_error = f(initial_error);

        RecoverableError {
            initial_error,
            recovery_error,
        }
    }

    pub(crate) fn into_error(self) -> RecoverableError<ImplError<E>>
    where
        E: PseudoError,
    {
        let Self {
            initial_error,
            recovery_error,
        } = self;

        RecoverableError {
            initial_error: initial_error.into_error(),
            recovery_error,
        }
    }
}

impl<E> From<E> for RecoverableError<E> {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl<E: fmt::Display> fmt::Display for RecoverableError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            initial_error,
            recovery_error,
        } = self;

        writeln!(f, "The operation encountered an error: {initial_error}")?;
        if let Some(recovery_error) = recovery_error {
            writeln!(f, "Recovering from the error failed: {recovery_error}")?;
        } else {
            writeln!(f, "The partial operation was successfully undone.")?;
        }

        Ok(())
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RecoverableError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.initial_error)
    }
}

#[derive(Debug, thiserror::Error)]
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
pub(crate) struct FatalErrorState(Result<(), FatalError>);

impl Default for FatalErrorState {
    fn default() -> Self {
        Self(Ok(()))
    }
}

impl FatalErrorState {
    pub(crate) fn check(&self) -> Result<(), FatalError> {
        self.0.clone()
    }

    /// Use [Result::map_err] to convert the error to a fatal error and record it in the [FatalErrorState].
    /// ```rust,ignore
    /// i32::from_str("not a number")
    ///     .map_err(fatal_error_state.into_fatal())
    /// ```
    pub(crate) fn make_fatal<E: Into<AnyError>>(&mut self) -> impl FnOnce(E) -> FatalError {
        |err| {
            // Only keep the first fatal error, because any further errors were born of an already faulty state
            if let Err(err) = self.check() {
                err
            } else {
                let err = FatalError::new(err.into());
                self.0 = Err(err.clone());
                err
            }
        }
    }
}

/// A fatal error which occurred on a [Client](crate::Client) or [Server](crate::Server),
/// due to an implementation error (or bug in spru).
/// Once [FatalError] is returned, any further operations will only return
/// fatal errors, as the state has been corrupted. A server must be
/// reloaded from a [Save](crate::server::Save) if available; a client must be
/// reseeded.
#[derive(Debug, Clone)]
pub struct FatalError {
    inner: std::sync::Arc<AnyError>,
}

impl FatalError {
    fn new(inner: AnyError) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl fmt::Display for FatalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The server or client is inoperable due to implementation error: {}",
            self.inner
        )
    }
}

impl std::error::Error for FatalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}
