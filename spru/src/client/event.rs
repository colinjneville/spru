use derive_where::derive_where;

#[derive_where(Debug; GameComplete<Client>)]
#[derive(derive_more::From)]
#[non_exhaustive]
pub enum Event<Client: super::Client> {
    GameComplete(GameComplete<Client>),
}

#[derive_where(Debug; Client::GameOutcome)]
pub struct GameComplete<Client: super::Client> {
    pub game_outcome: Client::GameOutcome,
}
