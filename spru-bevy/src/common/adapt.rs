use bevy::prelude;

pub fn map_err<E: std::error::Error + Send + Sync + 'static>(result: Result<(), E>) -> prelude::Result {
    result.map_err(Into::into)
}