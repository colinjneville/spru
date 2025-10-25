use std::{
    collections::{HashMap, VecDeque},
    fmt,
};

use derive_where::derive_where;
use rand::{Rng, SeedableRng};
use spru::player;

use crate::{Messaging, event};

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("Client {0} does not exist")]
    ClientDoesNotExist(player::Id),
}

pub trait Anyhow: std::error::Error + Send + Sync + 'static {}
impl<E: std::error::Error + Send + Sync + 'static> Anyhow for E {}

struct SyncServer<Server: spru::Server> {
    server: Server,
    add_player_requests: Vec<<Server::PlayerInit as spru::player::Init>::In>,
}

impl<Server: spru::Server> SyncServer<Server> {
    fn new(server: Server) -> Self {
        Self {
            server,
            add_player_requests: vec![],
        }
    }
}

enum SyncClientState<Client: spru::Client, Lookup> {
    Pending(spru::common::Seed<Client::Common>),
    Initialized(SyncClientInitialized<Client, Lookup>),
    // TODO: SyncClient should be broken into SyncClientState and a new SyncClientData
    Invalid,
}

struct SyncClientInitialized<Client: spru::Client, Lookup> {
    client: Client,
    outgoing_queue: VecDeque<spru::common::signal::ToServer<Client::Common>>,
    lookup: Lookup,
    game_outcome: Option<Client::GameOutcome>,
}

struct SyncClient<Client: spru::Client, Lookup> {
    incoming_queue: VecDeque<spru::common::signal::ToClient<Client::Common>>,
    user_incoming_queue: VecDeque<ClientCommand<Client>>,
    state: SyncClientState<Client, Lookup>,
}

impl<Client: spru::Client, Lookup> SyncClient<Client, Lookup> {
    pub fn new(init: spru::common::Seed<Client::Common>) -> Self {
        Self {
            incoming_queue: VecDeque::new(),
            user_incoming_queue: VecDeque::new(),
            state: SyncClientState::Pending(init),
        }
    }
}

/// A simple sync, single-threaded device for managing a Server and multiple Clients for tests.
/// One update is made from all possible pending updates at random, to exercise the order variance
/// found in actual async and networked games. Order is still maintained where it is guaranteed
/// (messages from the server to the same client, etc.).
pub struct SyncRunner<Server: spru::Server, Client: spru::Client, Lookup> {
    server: SyncServer<Server>,
    clients: HashMap<spru::player::Id, SyncClient<Client, Lookup>>,
    game_outcome: Option<Client::GameOutcome>,
    random: rand::rngs::StdRng,

    is_dirty: bool,
}

impl<Server, Client, Lookup> SyncRunner<Server, Client, Lookup>
where
    Server: spru::Server,
    Client: spru::Client<Common = Server::Common, GameOutcome: fmt::Debug + PartialEq + Clone>,
    Lookup: spru::item::Lookup<State = Client::State> + Default,
{
    pub fn new<GameInit>(
        game_init: GameInit,
        player_init: Server::PlayerInit,
        reaction: Server::Reaction,
    ) -> anyhow::Result<Self>
    where
        GameInit:
            spru::game::Init<State = Server::State, Action = Server::Action, Root = Server::Root>,
    {
        let spru_server = Server::init(game_init, player_init, reaction)?;
        let server = SyncServer::new(spru_server);
        let random = rand::rngs::StdRng::from_os_rng();
        Ok(Self {
            server,
            clients: HashMap::new(),
            game_outcome: None,
            random,
            is_dirty: true,
        })
    }

    pub fn add_player(
        &mut self,
        player: <Server::PlayerInit as spru::player::Init>::In,
    ) -> anyhow::Result<()> {
        self.is_dirty = true;

        self.server.add_player_requests.push(player);
        Ok(())
    }

    pub fn players(&self) -> impl Iterator<Item = player::Id> + '_ {
        self.clients.keys().copied()
    }

    pub fn stage_interaction(
        &mut self,
        player_id: player::Id,
        interaction: Client::Interaction,
    ) -> Result<(), RunnerError> {
        self.client_command(player_id, ClientCommand::StageInteraction(interaction))
    }

    pub fn apply_interactions(
        &mut self,
        player_id: player::Id,
        pending: Option<spru::interaction::Pending>,
    ) -> Result<(), RunnerError> {
        self.client_command(player_id, ClientCommand::ApplyInteractions(pending))
    }

    pub fn revert_interactions(
        &mut self,
        player_id: player::Id,
        pending: Option<spru::interaction::Pending>,
    ) -> Result<(), RunnerError> {
        self.client_command(player_id, ClientCommand::RevertInteractions(pending))
    }

    fn client_command(
        &mut self,
        player_id: player::Id,
        command: ClientCommand<Client>,
    ) -> Result<(), RunnerError> {
        let client = self.get_client_mut(player_id)?;
        client.user_incoming_queue.push_back(command);
        Ok(())
    }

    pub fn run_one(&mut self) -> anyhow::Result<Run<Server, Client>> {
        let mut messaging = Messaging::new();

        let mut picker = Picker::new(&mut self.random);

        for i in 0..self.server.add_player_requests.len() {
            picker.add_choice(Choice::AddPlayer(i));
        }

        for (&id, client) in &self.clients {
            // TODO this is kinda hacky
            if !client.incoming_queue.is_empty()
                || matches!(&client.state, SyncClientState::Pending(_))
            {
                picker.add_choice(Choice::Incoming(id));
            }
            if let SyncClientState::Initialized(initialized) = &client.state {
                if !initialized.outgoing_queue.is_empty() {
                    picker.add_choice(Choice::Outgoing(id));
                }
                if !client.user_incoming_queue.is_empty() {
                    picker.add_choice(Choice::UserCommand(id));
                }
            }
        }

        if let Some(choice) = picker.into_final_choice() {
            self.is_dirty = true;

            match choice {
                Choice::AddPlayer(index) => self.run_add_player(&mut messaging, index)?,
                Choice::Incoming(id) => self.run_incoming(&mut messaging, id)?,
                Choice::Outgoing(id) => self.run_outgoing(&mut messaging, id)?,
                Choice::UserCommand(id) => self.run_user_command(&mut messaging, id)?,
            }
            Ok(Run::Ran(messaging))
        } else {
            if self.is_dirty {
                self.check_consistent(self.game_outcome.is_some());
                self.is_dirty = false;
            }

            Ok(Run::Idle)
        }
    }

    fn check_consistent(&self, _is_complete: bool) {
        // TODO more checks

        for (id, client) in &self.clients {
            let SyncClientState::Initialized(initialized) = &client.state else {
                unreachable!("Runner is idle, but client {id} is uninitialized")
            };

            assert_eq!(
                initialized.game_outcome, self.game_outcome,
                "Runner is idle, but client {id}'s GameOutcome doesn't match the server's"
            );
        }
    }

    fn run_add_player(
        &mut self,
        messaging: &mut Messaging<Server, Client>,
        index: usize,
    ) -> anyhow::Result<()> {
        let player = self.server.add_player_requests.swap_remove(index);
        let spru::server::Output {
            outbound,
            events,
            ret: client_init,
        } = self.server.server.add_player(player)?;

        let player_id = client_init.local_player_id();

        messaging.record_event(event::PlayerConfirmed { player_id });

        self.clients.insert(player_id, SyncClient::new(client_init));
        self.queue_server_outbound(outbound);
        messaging.record_events(events);

        Ok(())
    }

    fn run_outgoing(
        &mut self,
        messaging: &mut Messaging<Server, Client>,
        client_id: spru::player::Id,
    ) -> anyhow::Result<()> {
        println!("run_outgoing {client_id}");

        let client = self.clients.get_mut(&client_id).unwrap();
        let SyncClientState::Initialized(initialized) = &mut client.state else {
            unreachable!("Uninitialized client has no outgoing directives")
        };
        let signal = initialized.outgoing_queue.pop_front().unwrap();

        let spru::server::Output {
            outbound,
            events,
            ret: (),
        } = self.server.server.apply_signal(client_id, signal)?;

        self.queue_server_outbound(outbound);
        messaging.record_events(events);

        Ok(())
    }

    fn run_incoming(
        &mut self,
        messaging: &mut Messaging<Server, Client>,
        client_id: player::Id,
    ) -> anyhow::Result<()> {
        println!("run_incoming {client_id}");

        let mut client = self.clients.remove(&client_id).unwrap();
        client.state = SyncClientState::Initialized(
            match std::mem::replace(&mut client.state, SyncClientState::Invalid) {
                SyncClientState::Pending(init) => self.run_pending_client(messaging, init)?,
                SyncClientState::Initialized(sync_client_initialized) => self
                    .run_initialized_client(
                        messaging,
                        client_id,
                        &mut client,
                        sync_client_initialized,
                    )?,
                SyncClientState::Invalid => unreachable!(),
            },
        );

        self.clients.insert(client_id, client);
        Ok(())
    }

    fn run_user_command(
        &mut self,
        messaging: &mut Messaging<Server, Client>,
        client_id: spru::player::Id,
    ) -> anyhow::Result<()> {
        println!("run_user_interaction {client_id}");

        let client = self.clients.get_mut(&client_id).unwrap();
        let SyncClientState::Initialized(initialized) = &mut client.state else {
            unreachable!("Uninitialized client can't run user command")
        };
        let command = client.user_incoming_queue.pop_front().unwrap();

        let outbound = match command {
            ClientCommand::StageInteraction(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: pending_interaction_id,
                } = initialized
                    .client
                    .stage_interaction(&mut initialized.lookup, arg)?;

                // TODO unify these
                let events = events.into_iter().map(|e| (client_id, e));
                messaging.record_events(events);

                messaging.record_event(event::InteractionStaged {
                    player_id: client_id,
                    pending_interaction_id,
                });

                outbound
            }
            ClientCommand::ApplyInteractions(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: (),
                } = initialized
                    .client
                    .apply_interactions(&mut initialized.lookup, arg)?;

                let events = events.into_iter().map(|e| (client_id, e));
                messaging.record_events(events);

                outbound
            }
            ClientCommand::RevertInteractions(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: (),
                } = initialized
                    .client
                    .revert_interactions(&mut initialized.lookup, arg)?;

                let events = events.into_iter().map(|e| (client_id, e));
                messaging.record_events(events);

                outbound
            }
        };

        Self::queue_client_outbound(initialized, outbound);

        Ok(())
    }

    fn run_initialized_client(
        &mut self,
        messaging: &mut Messaging<Server, Client>,
        client_id: spru::player::Id,
        client: &mut SyncClient<Client, Lookup>,
        mut initialized: SyncClientInitialized<Client, Lookup>,
    ) -> anyhow::Result<SyncClientInitialized<Client, Lookup>> {
        let directive = client.incoming_queue.pop_front().unwrap();
        let spru::client::Output {
            outbound,
            events,
            ret: (),
        } = initialized
            .client
            .signal(&mut initialized.lookup, directive)?;

        Self::queue_client_outbound(&mut initialized, outbound);
        let events = events.into_iter().map(|e| (client_id, e));
        messaging.record_events(events);

        Ok(initialized)
    }

    fn run_pending_client(
        &mut self,
        _messaging: &mut Messaging<Server, Client>,
        init: spru::common::Seed<Server::Common>,
    ) -> anyhow::Result<SyncClientInitialized<Client, Lookup>> {
        let mut lookup = Lookup::default();

        let client = Client::init(&mut lookup, init)?;

        let client = SyncClientInitialized {
            client,
            lookup,
            outgoing_queue: VecDeque::new(),
            game_outcome: None,
        };

        Ok(client)
    }

    fn queue_server_outbound(
        &mut self,
        outbound: impl IntoIterator<
            Item = (
                spru::player::Id,
                spru::common::signal::ToClient<Server::Common>,
            ),
        >,
    ) {
        let mut n = 0;
        for (id, signal) in outbound {
            n += 1;
            let client = self.clients.get_mut(&id).unwrap();
            client.incoming_queue.push_back(signal);
        }
        println!("enqueued {n} signals from server");
    }

    fn queue_client_outbound(
        client: &mut SyncClientInitialized<Client, Lookup>,
        outbound: impl IntoIterator<Item = spru::common::signal::ToServer<Server::Common>>,
    ) {
        for signal in outbound {
            client.outgoing_queue.push_back(signal);
        }
    }

    #[allow(dead_code)]
    fn get_client(&self, player_id: player::Id) -> Result<&SyncClient<Client, Lookup>, RunnerError> {
        self.clients.get(&player_id)
            .ok_or(RunnerError::ClientDoesNotExist(player_id))
    }

    fn get_client_mut(&mut self, player_id: player::Id) -> Result<&mut SyncClient<Client, Lookup>, RunnerError> {
        self.clients.get_mut(&player_id)
            .ok_or(RunnerError::ClientDoesNotExist(player_id))
    }
}

enum Choice {
    AddPlayer(usize),
    Incoming(spru::player::Id),
    Outgoing(spru::player::Id),
    UserCommand(spru::player::Id),
}

struct Picker<'r, T> {
    random: &'r mut rand::rngs::StdRng,
    choice: Option<T>,
    denominator: u32,
}

impl<'r, T> Picker<'r, T> {
    pub fn new(random: &'r mut rand::rngs::StdRng) -> Self {
        Self {
            random,
            choice: None,
            denominator: 0,
        }
    }

    pub fn add_choice(&mut self, choice: T) {
        self.denominator += 1;
        if self.random.random_ratio(1, self.denominator) {
            self.choice = Some(choice);
        }
    }

    pub fn into_final_choice(self) -> Option<T> {
        self.choice
    }
}

#[derive_where(Debug; Messaging<Server, Client>)]
pub enum Run<Server: spru::Server, Client: spru::Client> {
    Idle,
    Ran(Messaging<Server, Client>),
}

#[derive_where(Debug; Client::Interaction)]
enum ClientCommand<Client: spru::Client> {
    StageInteraction(Client::Interaction),
    ApplyInteractions(Option<spru::interaction::Pending>),
    RevertInteractions(Option<spru::interaction::Pending>),
}
