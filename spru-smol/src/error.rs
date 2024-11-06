#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Failed to serialize: {e}")]
pub struct SerializeError {
    #[from]
    e: rmp_serde::encode::Error,
}

impl SerializeError {
    pub(crate) fn new(e: rmp_serde::encode::Error) -> Self {
        Self {
            e,
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Failed to deserialize: {e}")]
pub struct DeserializeError {
    #[from]
    e: rmp_serde::decode::Error,
}

impl DeserializeError {
    pub(crate) fn new(e: rmp_serde::decode::Error) -> Self {
        Self {
            e,
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum IoSerializationError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialize(#[from] SerializeError),
    #[error(transparent)]
    Deserialize(#[from] DeserializeError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum IoSerializeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serialize(#[from] SerializeError),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum IoDeserializeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Deserialize(#[from] DeserializeError),
}
