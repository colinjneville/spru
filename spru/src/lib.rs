pub mod action;
pub use action::Action;
pub mod client;
pub use client::Client;
pub mod state;
pub use state::State;
pub mod game;
mod history;
pub use history::History;
pub mod interactor;
pub use interactor::Interactor;
pub mod item;
pub use item::Item;
pub mod interaction;
pub use interaction::Interaction;
pub mod log;
pub mod player;
pub mod reaction;
pub use reaction::Reaction;
pub mod record;
pub use record::Record;
pub mod save;
pub use save::Save;
pub mod server;
pub use server::Server;
pub mod snapshot;
pub use snapshot::Snapshot;
pub mod transaction;
pub use transaction::Transaction;
mod visibility;
pub use visibility::Visibility;
mod zone;
pub use zone::Zone;

pub use spru_macro::FromInfallible;

// TODO This needs a name other than `State` (the telety definition uses the macro namespace),
// but `ItemState` is not great.
pub use spru_macro::State as ItemState;

#[doc(hidden)]
pub mod __private {
    pub use telety;
    // pub use amass;
    pub use serde;

    // #[path = "../type_index.rs"]
    // pub mod type_index;

    // pub use crate::state::do_apply_state;
}

// TODO this is gross
// pub(crate) use __private::type_index;

pub trait Serial: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static { }

impl<T: Sized + serde::Serialize + serde::de::DeserializeOwned + 'static> Serial for T { }

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


#[derive(Debug, Default)]
pub struct CustomError<const ImplError: bool = false> {
    inner: Option<Box<dyn std::error::Error>>,
}

impl<const ImplError: bool> CustomError<ImplError> {
    pub fn new<E: std::error::Error + 'static>(e: E) -> Self {
        Self {
            inner: Some(Box::new(e)),
        }
    }

    pub fn get(&self) -> Option<&dyn std::error::Error> {
        self.inner.as_ref()
            .map(|e| &**e)
    }

    pub fn try_cast<E: std::error::Error + 'static>(self) -> Result<E, Self> {
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

impl CustomError<false> {
    pub fn into_error(self) -> CustomError<true> {
        let Self {
            inner,
        } = self;

        CustomError {
            inner,
        }
    }
}

impl<E: std::error::Error + 'static> From<E> for CustomError<false> {
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl<const ImplError: bool> std::fmt::Display for CustomError<ImplError> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(inner) = &self.inner {
            std::fmt::Display::fmt(inner, f)?;
        } else {
            write!(f, "Generic Error")?;
        }

        Ok(())
    }
}

impl std::error::Error for CustomError<true> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.get()
            .map(std::error::Error::source)
            .flatten()
    }
}
