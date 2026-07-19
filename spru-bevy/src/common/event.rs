use bevy::prelude;

// TODO not yet used
/// A fatal error occurred on a client or server.  
/// A server must reload from a [spru::server::Save], a client must
/// reseed from the server.
#[derive(Debug)]
#[derive(prelude::EntityEvent)]
struct FatalError {
    /// The client or server entity the fatal error occurred on
    pub entity: prelude::Entity,
    /// The error 
    pub error: spru::common::error::FatalError,
}