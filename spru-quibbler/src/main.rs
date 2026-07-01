#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod actions;
use std::fmt;

pub use actions::Actions;
pub mod data;
pub mod game;
pub mod interaction;
pub use interaction::Interaction;
mod lexicon;
pub use lexicon::Lexicon;
pub mod script;
mod play;
pub mod round;
pub use play::Play;
mod player;
mod plugin;
mod reaction;
// pub use reaction::Reaction;
mod state;
pub use state::State;

use bevy::prelude;

type Language = spru_script_rhai::RhaiInstance::<self::Lexicon>;
type GameInit = spru_script::GameInit<spru::item::IdT<game::Root>, Language, ()>;
type PlayerInit = spru_script::PlayerInit<spru::item::IdT<game::Root>, Language, player::Input>;
type Client = spru::client::Impl<Interaction, game::Outcome>;

type Reaction = spru_script::Reaction<spru::item::IdT<game::Root>, reaction::Trigger, game::Outcome, Language>;
type Server = spru::server::Impl<Interaction, Reaction, PlayerInit>;
type Common = <Client as spru::Client>::Common;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, prelude::States)]
enum AppState {
    #[default]
    MainMenu,
    
    Config,

    InGame,
}

fn main() {
    #[rustfmt::skip]
    let _app_exit = prelude::App::new()
        .add_plugins((
            plugin::Core,
            plugin::Ui,
            plugin::Server,
            plugin::Client,
            plugin::Local,
            plugin::Remote,
            plugin::RemoteClient,
            plugin::RemoteServer,
        ))
        .run();
}

#[derive(Debug, Default)]
#[derive(prelude::Component)]
struct Log {
    log: Vec<String>,
}

impl Log {
    fn log<S: ToString>(&mut self, message: S) {
        self.log.push(message.to_string());
    }

    fn try_log<S: ToString>(log: &mut Option<prelude::Mut<'_, Self>>, message: S) {
        if let Some(log) = log {
            log.log(message);
        } else {
            prelude::warn!("Log not available: {}", message.to_string());
        }
    }

    fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &str> {
        self.log
            .iter()
            .map(String::as_str)
    }
}

#[derive(Debug)]
pub struct Error(pub anyhow::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

macro_rules! anyhow {
    ($msg:literal $(,)?) => {
        $crate::Error(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        $crate::Error(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Error(anyhow::anyhow!($fmt, $($arg)*))
    }
}
pub(crate) use anyhow;

macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::anyhow!($msg).into());
    };
    ($err:expr $(,)?) => {
        return Err($crate::::anyhow!($err).into());
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::::anyhow!($fmt, $($arg)*).into());
    }
}
pub(crate) use bail;

fn error_to_console(prelude::In(result): prelude::In<prelude::Result>) {
    if let Err(err) = result {
        prelude::warn!("{err}");
    }
}
