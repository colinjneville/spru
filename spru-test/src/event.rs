use derive_where::derive_where;

#[derive_where(Debug; Event<Server, Client>)]
#[derive_where(Default)]
pub struct Messaging<Server: spru::Server, Client: spru::Client> {
    events: Vec<Event<Server, Client>>
}

impl<Server: spru::Server, Client: spru::Client> Messaging<Server, Client> {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_event<E: Into<Event<Server, Client>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn record_events<E: Into<Event<Server, Client>>>(&mut self, events: impl IntoIterator<Item = E>) {
        for event in events {
            self.record_event(event);
        }
    }
}

impl<Server: spru::Server, Client: spru::Client> IntoIterator for Messaging<Server, Client> {
    type Item = Event<Server, Client>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

#[derive_where(Debug; ServerEvent<Server>, ClientEvent<Client>)]
#[derive(derive_more::From)]
pub enum Event<Server: spru::Server, Client: spru::Client> {
    PlayerConfirmed(PlayerConfirmed),
    InteractionStaged(InteractionStaged),
    ServerEvent(ServerEvent<Server>),
    ClientEvent(ClientEvent<Client>),
}

impl<Server: spru::Server, Client: spru::Client> From<spru::server::Event<Server>> for Event<Server, Client> {
    fn from(event: spru::server::Event<Server>) -> Self {
        Self::ServerEvent(ServerEvent { event })
    }
}

impl<Server: spru::Server, Client: spru::Client> From<(spru::player::Id, spru::client::Event<Client>)> for Event<Server, Client> {
    fn from((player_id, event): (spru::player::Id, spru::client::Event<Client>)) -> Self {
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
    pub pending_interaction_id: spru::interaction::Pending,
}

#[derive_where(Debug; spru::server::Event<Server>)]
pub struct ServerEvent<Server: spru::Server> {
    pub event: spru::server::Event<Server>,
}

#[derive_where(Debug; spru::client::Event<Client>)]
pub struct ClientEvent<Client: spru::Client> {
    pub player_id: spru::player::Id,
    pub event: spru::client::Event<Client>,
}
