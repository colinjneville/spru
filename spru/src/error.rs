use std::fmt;

use crate::{action, player};

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Player {player} has desynced due to implementation error: {message}")]
pub struct SyncError {
    player: player::Id,
    message: String,
}

impl SyncError {
    pub fn new<S: Into<String>>(player: player::Id, message: S) -> Self {
        let message = message.into();
        Self {
            player,
            message,
        }
    }
}

// TODO actual errors
#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Error!:\n{0}")]
// r#Backtrace avoids thiserror's special Backtrace handling which requires nightly
pub struct TempError(std::backtrace::r#Backtrace);

impl TempError {
    #[track_caller]
    pub fn new() -> Self {
        Self(std::backtrace::Backtrace::force_capture())
    }

    #[track_caller]
    pub fn discard<T>(_t: T) -> Self {
        Self::new()
    }
}

// TODO
pub type TempResult<T> = std::result::Result<T, TempError>;


#[derive(Debug, Default)]
pub struct AnyError {
    inner: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl AnyError {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(e: E) -> Self {
        Self {
            inner: Some(Box::new(e)),
        }
    }

    pub fn get(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        self.inner.as_ref()
            .map(|e| &**e)
    }

    pub fn try_cast<E: std::error::Error + Send + Sync + 'static>(self) -> Result<E, Self> {
        let Self {
            mut inner,
        } = self;

        if let Some(some_inner) = inner {
            match some_inner.downcast() {
                Ok(e) => return Ok(*e),
                Err(e) => inner = Some(e),
            }
        }

        Err(Self {
            inner,
        })
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for AnyError {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = &self.inner {
            std::fmt::Display::fmt(inner, f)?;
        } else {
            write!(f, "Generic Error")?;
        }

        Ok(())
    }
}

pub type AnyResult<T> = std::result::Result<T, AnyError>;

impl PsuedoError for AnyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.get()
            .map(std::error::Error::source)
            .flatten()
    }
}

pub trait PsuedoError: std::fmt::Debug + std::fmt::Display {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>;

    fn into_error(self) -> ImplError<Self>
    where 
        Self: Sized 
    {
        ImplError::new(self)
    }

    fn as_error(&self) -> &ImplError<Self> {
        ImplError::new_ref(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImplError<E: ?Sized>(E);

impl<E> ImplError<E> {
    pub fn new(e: E) -> Self {
        Self(e)
    }
}

impl<E: ?Sized> ImplError<E> {
    pub fn new_ref(e: &E) -> &Self {
        // SAFETY: transparent struct refs can be transmuted safely
        unsafe {
            std::mem::transmute(e)
        }
    }
}

impl<E: PsuedoError> std::fmt::Display for ImplError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl<E: PsuedoError> std::error::Error for ImplError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        PsuedoError::source(&self.0)
    }
}

#[derive(Debug)]
pub struct RecoverableError<E> {
    pub initial_error: E,
    pub recovery_error: Option<action::Error>,
}

impl<E> RecoverableError<E> {
    pub(crate) fn new(initial_error: E) -> Self {
        Self {
            initial_error,
            recovery_error: None,
        }
    }

    pub(crate) fn set_recovery_error(&mut self, recovery_error: action::Error) {
        self.recovery_error = Some(recovery_error);
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
        F: FnOnce(E) -> E2
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

pub type RecoverableResult<T, E> = std::result::Result<T, RecoverableError<E>>;
