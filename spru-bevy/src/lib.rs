#![deny(missing_debug_implementations)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

#[cfg(feature = "client")]
pub mod client;
pub mod common;
#[cfg(all(feature = "server", feature = "client"))]
pub mod local;
#[cfg(feature = "server")]
pub mod server;
