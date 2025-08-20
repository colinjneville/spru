use derive_where::derive_where;

#[derive(Debug)]
#[derive_where(Default)]
pub struct Messaging<GameOutcome> {
    events: Vec<Event<GameOutcome>>
}

impl<GameOutcome> Messaging<GameOutcome> {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_event<E: Into<Event<GameOutcome>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn record_events<E: Into<Event<GameOutcome>>>(&mut self, events: impl IntoIterator<Item = E>) {
        for event in events {
            self.record_event(event);
        }
    }
}

impl<GameOutcome> IntoIterator for Messaging<GameOutcome> {
    type Item = Event<GameOutcome>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

#[derive(Debug)]
#[derive(derive_more::From)]
pub enum Event<GameOutcome> {
    PlayerConfirmed(PlayerConfirmed),
    InteractionStaged(InteractionStaged),
    ServerEvent(ServerEvent<GameOutcome>),
    ClientEvent(ClientEvent<GameOutcome>),
}

impl<GameOutcome> From<spru::server::Event<GameOutcome>> for Event<GameOutcome> {
    fn from(event: spru::server::Event<GameOutcome>) -> Self {
        Self::ServerEvent(ServerEvent { event })
    }
}

impl<GameOutcome> From<(spru::player::Id, spru::client::Event<GameOutcome>)> for Event<GameOutcome> {
    fn from((player_id, event): (spru::player::Id, spru::client::Event<GameOutcome>)) -> Self {
        Self::ClientEvent(ClientEvent { player_id, event })
    }
}

#[derive(Debug)]
pub struct PlayerConfirmed {
    pub player_id: spru::player::Id,
}

#[derive(Debug)]
pub struct InteractionStaged {
    pub player_id: spru::player::Id,
    pub pending_transaction_id: spru::transaction::Pending,
}

#[derive(Debug)]
pub struct ServerEvent<GameOutcome> {
    pub event: spru::server::Event<GameOutcome>,
}

#[derive(Debug)]
pub struct ClientEvent<GameOutcome> {
    pub player_id: spru::player::Id,
    pub event: spru::client::Event<GameOutcome>,
}
