//!
//!
//! Add included plugins as required:  
//! ```rust,ignore
//! type MyServer = spru::server::Impl< /* ... */ >;
//! type MyClient = spru::client::Impl< /* ... */ >;
//!
//! bevy::app::App::new()
//! // ...
//!     .add_plugins((
//!         // Add the client plugin if any Clients will run on this App
//!         spru_bevy::client::Plugin::<MyClient>::default(),
//!         // Add the client plugin if any Servers will run on this App
//!         spru_bevy::server::Plugin::<MyServer>::default(),
//!         // Add the local plugin to automatically route signals
//!         // between local clients and servers in this App
//!         spru_bevy::local::Plugin::<MyServer, MyClient>::default(),
//!     ))
//! // ...
//! ```
//!
//! ## Creating Servers
//! Servers are created by using a command:
//! ```rust,ignore
//! commands.queue(spru_bevy::server::command::Init::<MyServer, _> {
//!     // ...
//! });
//! ```
//! The [local::Plugin] automatically creates added players as new Clients in the same [World](bevy::prelude::World).  
//! If you are not using the plugin, dequeue [Seed](spru::common::Seed)s from the
//! [PendingClients](server::component::PendingClients) component, and use the [client::command::Init]
//! command to initialize the Client in the desired location.
//!
//! ## IDs
//! Because multiple clients and servers can coexist in the same App, both have a  
//! [GameId](common::component::GameId) component, and clients have a [ClientId](client::component::ClientId).
//! You can filter a query to a client or server with matching ids using [ServerSSS::filter](server::ServerSSS::filter)
//! [(_mut)](server::ServerSSS::filter_mut) and [ClientSSS::filter](client::ClientSSS::filter)
//! [(_mut)](client::ClientSSS::filter_mut) respectively.
//!
//! ## Controlling Servers and Clients
//! Servers and Clients are controlled using the [server::component::FromUser] and [client::component::FromUser] components
//! attached to the server/client Entity. These are processed in an exclusive system, and fire [event](client::event)s
//! when run.
//!
//! ```rust,ignore
//! // Add an observer to keep track of all client ids
//! .add_observer(
//!         |client_init: prelude::On<spru_bevy::client::event::Init<MyClient>>,
//!          mut client_ids: prelude::ResMut<MyClientIds>|
//!          -> prelude::Result {
//!             let client_id = *client_init.result.as_ref().map_err(ToString::to_string)?;
//!             client_ids.0.push(client_id);
//!             Ok(())
//!         },
//!     )
//! ```
//!

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
