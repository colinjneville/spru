use crate::item::lookup;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Save {
    #[error(transparent)]
    Serialization(#[from] rmp_serde::encode::Error),
}

#[doc(hidden)]
#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Load {
    #[error(transparent)]
    Deserialization(#[from] rmp_serde::decode::Error),
    #[error("{0}")]
    Lookup(lookup::Error),
}

impl From<lookup::Error> for Load {
    fn from(value: lookup::Error) -> Self {
        Self::Lookup(value)
    }
}