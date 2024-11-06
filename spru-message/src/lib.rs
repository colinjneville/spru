pub mod header;
pub use header::Header;
pub mod message;
pub use message::Message;
pub mod payload;
pub use payload::Payload;
pub use spru_macro::payload_variant;

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Failed to serialize: {e}")]
pub struct SerializeError {
    #[from]
    e: rmp_serde::encode::Error,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Failed to deserialize: {e}")]
pub struct DeserializeError {
    #[from]
    e: rmp_serde::decode::Error,
}



#[cfg(test)]
mod test {
    use super::*;

    #[crate::payload_variant(0 => Variant0)]
    #[crate::payload_variant(1 => Variant1)]
    pub struct Pload;

    #[derive(Debug)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Variant0;

    #[derive(Debug)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Variant1;

    #[test]
    fn payload_variants() {
        let message0 = Message::<Pload>::new_raw(Variant0);
        let message1 = Message::<Pload>::new_raw(Variant1);

        assert!(message0.into_variant::<Variant0>().is_ok());
        assert!(message1.into_variant::<Variant1>().is_ok());

        let message0 = Message::<Pload>::new_raw(Variant0);
        let message1 = Message::<Pload>::new_raw(Variant1);

        assert!(message0.into_variant::<Variant1>().is_err());
        assert!(message1.into_variant::<Variant0>().is_err());
    }
}
