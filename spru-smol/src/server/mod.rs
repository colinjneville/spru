pub enum Payload<PlayerInfo> {
    Lobby(crate::lobby::server::Variant<PlayerInfo>),
    InGame(),
}