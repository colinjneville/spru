#[spru_message::payload_variant(0 => spru_smol::lobby::client::Variant::<MemberInfo>)]
#[spru_message::payload_variant(1 => spru::communication::Client::<PlayerData, Actions, GameOutcome>)]
#[spru_message::payload_variant(2 => spru_smol::lobby::server::Variant::<MemberInfo>)]
#[spru_message::payload_variant(3 => spru::communication::Server::<Interaction>)]
pub struct Payload;