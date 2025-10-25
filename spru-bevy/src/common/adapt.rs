use bevy::prelude;

pub fn map_err<E: std::error::Error + Send + Sync + 'static>(
    prelude::In(result): prelude::In<Result<(), E>>,
) -> prelude::Result {
    result.map_err(Into::into)
}
