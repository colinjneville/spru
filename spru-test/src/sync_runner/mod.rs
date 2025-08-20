use std::{collections::{HashMap, VecDeque}, fmt};

use rand::{Rng, SeedableRng};
use spru::player;

use crate::{event, Messaging};

pub trait Anyhow: std::error::Error + Send + Sync + 'static { }
impl<E: std::error::Error + Send + Sync + 'static> Anyhow for E { }

struct SyncServer<State, Action, Root, PlayerInit, PlayerInitIn, Interaction, Reaction> {
    server: spru::Server<State, Action, Root, PlayerInit, Interaction, Reaction>,
    add_player_requests: Vec<spru::server::add_player::Arg<PlayerInitIn>>,
}

impl<State, Action, Root, PlayerInit, PlayerInitIn, Interaction, Reaction> SyncServer<State, Action, Root, PlayerInit, PlayerInitIn, Interaction, Reaction> {
    fn new(server: spru::Server<State, Action, Root, PlayerInit, Interaction, Reaction>) -> Self {
        Self {
            server,
            add_player_requests: vec![],
        }
    }
}

enum SyncClientState<State, Action, Root, Interaction, GameOutcome, Lookup> {
    Pending(spru::client::init::Arg<State, Action, Root>),
    Initialized(SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup>),
    // TODO: SyncClient should be broken into SyncClientState and a new SyncClientData
    Invalid,
}

struct SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup> {
    client: spru::Client<Action, Root, Interaction, GameOutcome>,
    outgoing_queue: VecDeque<spru::server::signal::Arg<Interaction>>,
    lookup: Lookup,
    game_outcome: Option<GameOutcome>,
}

struct SyncClient<State, Action, Root, Interaction, GameOutcome, Lookup> {
    incoming_queue: VecDeque<spru::client::signal::Arg<Action, GameOutcome>>,
    user_incoming_queue: VecDeque<ClientArg<Interaction>>,
    state: SyncClientState<State, Action, Root, Interaction, GameOutcome, Lookup>,
}

impl<State, Action, Root, Interaction, GameOutcome, Lookup> SyncClient<State, Action, Root, Interaction, GameOutcome, Lookup> {
    pub fn new(init: spru::client::init::Arg<State, Action, Root>) -> Self {
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
pub struct SyncRunner<State, Action, Root, PlayerInit, PlayerInitIn, Interaction, Reaction, GameOutcome, Lookup = spru_util::lookup::Standalone> {
    server: SyncServer<State, Action, Root, PlayerInit, PlayerInitIn, Interaction, Reaction>,
    clients: HashMap<spru::player::Id, SyncClient<State, Action, Root, Interaction, GameOutcome, Lookup>>,
    game_outcome: Option<GameOutcome>,
    random: rand::rngs::StdRng,

    is_dirty: bool,
}

impl<State, Action, Root, PlayerInit, Interaction, Reaction, GameOutcome, Lookup> 
    SyncRunner<State, Action, Root, PlayerInit, PlayerInit::In, Interaction, Reaction, GameOutcome, Lookup> 
where
    Lookup: spru::item::Lookup<Error: Anyhow> + Default,
    State: 
        spru::State<spru::item::lookup::Canonical<State>> +
        spru::State<Lookup> +
        tagset::TagSet<Repr: TryFrom<u32>>,
    Action: 
        Clone + 
        spru::action::Base<Undo = Action> +
        spru::Action<spru::item::lookup::Canonical<State>, Error: Anyhow> +
        spru::Action<Lookup, Error: Anyhow>,
    Root: Clone,
    PlayerInit: spru::player::Init<State = State, Action = Action, Root = Root, Error: Anyhow>,
    Interaction: spru::Interaction<Action = Action, Root = Root>,
    Reaction: spru::Reaction<State = State, Action = Action, Root = Root, Trigger = Interaction::Trigger, GameOutcome = GameOutcome>,
    GameOutcome: fmt::Debug + PartialEq + Clone,
{
    pub fn new<GameInit>(
        game_init: GameInit, 
        player_init: PlayerInit,
        reaction: Reaction,
    ) -> anyhow::Result<Self>
    where 
        GameInit: spru::game::Init<State = State, Action = Action, Root = Root, Error: Anyhow>,
    {
        let spru_server = spru::Server::new(game_init, player_init, reaction)?;
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

    pub fn add_player(&mut self, player: spru::server::add_player::Arg<PlayerInit::In>) -> anyhow::Result<()> {
        self.is_dirty = true;

        self.server.add_player_requests.push(player);
        Ok(())
    }

    pub fn players(&self) -> impl Iterator<Item = player::Id> + '_ {
        self.clients.keys().copied()
    }

    pub fn client_command<Arg>(&mut self, player_id: player::Id, arg: Arg) -> Result<(), ()> 
    where
        Arg: Into<ClientArg<Interaction>>,
    {
        if let Some(client) = self.clients.get_mut(&player_id) {
            client.user_incoming_queue.push_back(arg.into());
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn run_one(&mut self) -> anyhow::Result<Run<GameOutcome>> {
        let mut events = Messaging::new();

        let mut picker = Picker::new(&mut self.random);

        for i in 0..self.server.add_player_requests.len() {
            picker.add_choice(Choice::AddPlayer(i));
        }
        
        for (&id, client) in &self.clients {
            // TODO this is kinda hacky
            if !client.incoming_queue.is_empty() || matches!(&client.state, SyncClientState::Pending(_)) {
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
                Choice::AddPlayer(index) => self.run_add_player(&mut events, index)?,
                Choice::Incoming(id) => self.run_incoming(&mut events, id)?,
                Choice::Outgoing(id) => self.run_outgoing(&mut events, id)?,
                Choice::UserCommand(id) => self.run_user_command(&mut events, id)?,
            }
            Ok(Run::Ran(events))
        } else {
            if self.is_dirty {
                self.check_consistent(self.game_outcome.is_some());
                self.is_dirty = false;
            }

            Ok(Run::Idle)
        }
    }

    fn check_consistent(&self, is_complete: bool) {
        // TODO more checks
        
        for (id, client) in &self.clients {
            let SyncClientState::Initialized(initialized) = &client.state
                else { unreachable!("Runner is idle, but client {id} is uninitialized") };

            assert_eq!(initialized.game_outcome, self.game_outcome, "Runner is idle, but client {id}'s GameOutcome doesn't match the server's");
        }
    }

    fn run_add_player(&mut self, state: &mut Messaging<GameOutcome>, index: usize) -> anyhow::Result<()> {
        let player = self.server.add_player_requests.swap_remove(index);
        let spru::server::Output {
            outbound,
            events,
            ret: spru::server::add_player::Ret {
                client_init,
                player_id,
            },
        } = self.server.server.add_player(player)?;

        state.record_event(event::PlayerConfirmed {
            player_id,
        });

        self.clients.insert(player_id, SyncClient::new(client_init));
        self.queue_server_outbound(outbound);
        state.record_events(events);

        Ok(())
    }

    fn run_outgoing(&mut self, state: &mut Messaging<GameOutcome>, client_id: spru::player::Id) -> anyhow::Result<()> {
        println!("run_outgoing {client_id}");

        let client = self.clients.get_mut(&client_id).unwrap();
        let SyncClientState::Initialized(initialized) = &mut client.state
            else { unreachable!("Uninitialized client has no outgoing directives") };
        let signal = initialized.outgoing_queue.pop_front().unwrap();

        let spru::server::Output {
            outbound,
            events,
            ret: spru::server::signal::Ret {
                
            },
        } = self.server.server.apply_signal(client_id, signal)?;

        self.queue_server_outbound(outbound);
        state.record_events(events);

        Ok(())
    }

    fn run_incoming(&mut self, messaging: &mut Messaging<GameOutcome>, client_id: player::Id) -> anyhow::Result<()> {
        println!("run_incoming {client_id}");

        let mut client = self.clients.remove(&client_id).unwrap();
        client.state = SyncClientState::Initialized(match std::mem::replace(&mut client.state, SyncClientState::Invalid) {
            SyncClientState::Pending(init) => self.run_pending_client(messaging, init)?,
            SyncClientState::Initialized(sync_client_initialized) 
                => self.run_initialized_client(messaging, client_id, &mut client, sync_client_initialized)?,
            SyncClientState::Invalid => unreachable!(),
        });

        self.clients.insert(client_id, client);
        Ok(())
    }

    fn run_user_command(&mut self, messaging: &mut Messaging<GameOutcome>, client_id: spru::player::Id) -> anyhow::Result<()> {
        println!("run_user_interaction {client_id}");

        let client = self.clients.get_mut(&client_id).unwrap();
        let SyncClientState::Initialized(initialized) = &mut client.state
            else { unreachable!("Uninitialized client can't run user command") };
        let command = client.user_incoming_queue.pop_front().unwrap();
        
        let outbound = match command {
            ClientArg::StageInteraction(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: spru::client::stage_interaction::Ret {
                        pending_transaction_id,
                    }
                } = initialized.client.stage_interaction(&mut initialized.lookup, arg)?;

                // TODO unify these
                let events = events.into_iter()
                    .map(|e| (client_id, e));
                messaging.record_events(events);

                messaging.record_event(event::InteractionStaged {
                    player_id: client_id,
                    pending_transaction_id,
                });

                outbound
            }
            ClientArg::ApplyInteraction(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: spru::client::apply_interaction::Ret {

                    },
                } = initialized.client.apply_interaction(&mut initialized.lookup, arg)?;

                let events = events.into_iter()
                    .map(|e| (client_id, e));
                messaging.record_events(events);

                outbound
            }
            ClientArg::RevertInteraction(arg) => {
                let spru::client::Output {
                    outbound,
                    events,
                    ret: spru::client::revert_interaction::Ret {
                        
                    },
                } = initialized.client.revert_interaction(&mut initialized.lookup, arg)?;

                let events = events.into_iter()
                    .map(|e| (client_id, e));
                messaging.record_events(events);

                outbound
            }
        };
        
        Self::queue_client_outbound(initialized, outbound);

        Ok(())
    }

    fn run_initialized_client(&mut self, messaging: &mut Messaging<GameOutcome>, client_id: spru::player::Id, client: &mut SyncClient<State, Action, Root, Interaction, GameOutcome, Lookup>, mut initialized: SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup>)
        -> anyhow::Result<SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup>>
    {
        let directive = client.incoming_queue.pop_front().unwrap();
        let spru::client::Output {
            outbound,
            events,
            ret: spru::client::signal::Ret {

            },
        } = initialized.client.signal(&mut initialized.lookup, directive)?;

        Self::queue_client_outbound(&mut initialized, outbound);
        let events = events.into_iter()
            .map(|e| (client_id, e));
        messaging.record_events(events);

        Ok(initialized)
    }

    fn run_pending_client(&mut self, _messaging: &mut Messaging<GameOutcome>, init: spru::client::init::Arg<State, Action, Root>) 
        -> anyhow::Result<SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup>> 
    {
        let mut lookup = Lookup::default();

        let client = spru::Client::init(&mut lookup, init)?;

        let client = SyncClientInitialized {
            client,
            lookup,
            outgoing_queue: VecDeque::new(),
            game_outcome: None,
        };

        Ok(client)
    }

    fn queue_server_outbound(&mut self, outbound: impl IntoIterator<Item = (spru::player::Id, spru::client::signal::Arg<Action, GameOutcome>)>) {
        for (id, signal) in outbound {
            let client = self.clients.get_mut(&id).unwrap();
            client.incoming_queue.push_back(signal);
        }
    }

    fn queue_client_outbound(client: &mut SyncClientInitialized<Action, Root, Interaction, GameOutcome, Lookup>, outbound: impl IntoIterator<Item = spru::server::signal::Arg<Interaction>>) {
        for signal in outbound {
            client.outgoing_queue.push_back(signal);
        }
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

#[derive(Debug)]
pub enum Run<GameOutcome> {
    Idle,
    Ran(Messaging<GameOutcome>),
}

#[derive(Debug)]
enum ClientArg<Interaction> {
    StageInteraction(spru::client::stage_interaction::Arg<Interaction>),
    ApplyInteraction(spru::client::apply_interaction::Arg),
    RevertInteraction(spru::client::revert_interaction::Arg),
}

impl<Interaction> From<spru::client::stage_interaction::Arg<Interaction>> for ClientArg<Interaction> {
    fn from(value: spru::client::stage_interaction::Arg<Interaction>) -> Self {
        Self::StageInteraction(value)
    }
}

impl<Interaction> From<spru::client::apply_interaction::Arg> for ClientArg<Interaction> {
    fn from(value: spru::client::apply_interaction::Arg) -> Self {
        Self::ApplyInteraction(value)
    }
}

impl<Interaction> From<spru::client::revert_interaction::Arg> for ClientArg<Interaction> {
    fn from(value: spru::client::revert_interaction::Arg) -> Self {
        Self::RevertInteraction(value)
    }
}

