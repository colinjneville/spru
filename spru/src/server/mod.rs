pub mod add_player;
pub mod event;
pub use event::Event;
pub mod signal;

use crate::{action, client, init, interaction::{self, Interactor}, item::{self, lookup::{canonical, Canonical}}, log, player, snapshot, transaction::{self, Transactions}, visibility, Init, Save, Snapshot, Transaction};

#[derive(Debug)]
pub struct Output<ActionCatalog, GameOutcome, Ret> {
    pub outbound: Vec<(player::Id, client::signal::Arg<ActionCatalog, GameOutcome>)>,
    pub events: Vec<Event<GameOutcome>>,
    pub ret: Ret,
}

#[derive(Debug)]
struct State<ActionCatalog, GameOutcome> {
    pub outbound: Vec<(player::Id, client::signal::Arg<ActionCatalog, GameOutcome>)>,
    pub events: Vec<Event<GameOutcome>>,
}

impl<ActionCatalog, GameOutcome> State<ActionCatalog, GameOutcome> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<client::signal::Internal<ActionCatalog, GameOutcome>>>(&mut self, player_id: player::Id, signal: S) {
        self.outbound.push((player_id, client::signal::Arg { signal: signal.into() }));
    }

    pub fn push_event<E: Into<Event<GameOutcome>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<ActionCatalog, GameOutcome, Ret> {
        let Self {
            outbound,
            events,
        } = self;

        Output {
            outbound,
            events,
            ret,
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum NewError<GameInitError, ActionsError> {
    #[error(transparent)]
    Lookup(#[from] canonical::Error),
    #[error(transparent)]
    Init(GameInitError),
    #[error(transparent)]
    Log(#[from] log::Error<canonical::Error, ActionsError>)
}

impl<GameInitError, ActionsError> From<init::Error<GameInitError>> for NewError<GameInitError, ActionsError> {
    fn from(value: init::Error<GameInitError>) -> Self {
        match value {
            init::Error::Lookup(e) => Self::Lookup(e),
            init::Error::Init(e) => Self::Init(e),
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Snapshot(#[from] snapshot::ApplyError<canonical::Error>),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum AddPlayerError<PlayerInitError, ActionsError> {
    #[error(transparent)]
    Snapshot(#[from] snapshot::CreateError),
    #[error(transparent)]
    Lookup(#[from] canonical::Error), 
    #[error(transparent)]
    Init(PlayerInitError),
    #[error(transparent)]
    Log(#[from] log::Error<canonical::Error, ActionsError>),
}

impl<PlayerInitError, ActionsError> From<player::manager::InitializeError<PlayerInitError>> for AddPlayerError<PlayerInitError, ActionsError> {
    fn from(value: player::manager::InitializeError<PlayerInitError>) -> Self {
        match value {
            player::manager::InitializeError::Lookup(e) => Self::Lookup(e),
            player::manager::InitializeError::Init(e) => Self::Init(e),
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ApplyInteractionError<ActionsError, InteractionError> {
    Lookup(#[from] canonical::Error), 
    Log(#[from] log::Error<canonical::Error, ActionsError>),
    Interaction(InteractionError),
    /// Interaction uses older versions than on the server
    Stale,
}

impl<ActionError, InteractionError> From<interaction::Error<canonical::Error, InteractionError>> for ApplyInteractionError<ActionError, InteractionError> {
    fn from(value: interaction::Error<canonical::Error, InteractionError>) -> Self {
        match value {
            interaction::Error::Lookup(e) => Self::Lookup(e),
            interaction::Error::Interaction(e) => Self::Interaction(e),
        }
    }
}

impl<ActionError, InteractionError> From<interaction::reaction::Error> for ApplyInteractionError<ActionError, InteractionError> {
    fn from(value: interaction::reaction::Error) -> Self {
        match value {
            interaction::reaction::Error::Lookup(e) => Self::Lookup(e),
        }
    }
}

#[derive(Debug)]
pub struct Server<ItemCatalog, ActionCatalog, Root, PlayerInit, Reaction> {
    lookup: item::lookup::Canonical<ItemCatalog>,
    root: item::IdT<Root>,
    player_manager: player::Manager<PlayerInit>,
    log: log::Server<ActionCatalog>,
    visibility: visibility::Manager,
    reservation: item::id::Reservation,
    reaction: Reaction,
}

impl<ItemCatalog, ActionCatalog, Root, PlayerInit, Reaction> Server<ItemCatalog, ActionCatalog, Root, PlayerInit, Reaction> {
    pub fn new<GameInit>(
        game_init: GameInit, 
        input: GameInit::In, 
        player_init: PlayerInit,
        reaction: Reaction,
    ) -> Result<Self, NewError<GameInit::Error, ActionCatalog::Error>> 
    where 
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>>,
        GameInit: Init<ItemCatalog, ActionCatalog, Root, Out = item::IdT<Root>>,
    {
        let player_manager = player::Manager::new(player_init);

        let reservation = item::id::Reservation::all();

        let mut lookup = Canonical::new();

        // TODO 
        // Nothing exists yet, but we need to give the interactor an id for our Root (which we are creating now...).
        // Give an invalid Id, and instruct implementers to resist the urge to use it (or we error at runtime).
        let mut interactor = Interactor::new(&lookup, &reservation, item::Id::new().force_type());
        let root = game_init.initialize(&mut interactor, input)?;

        let mut log = log::Server::new();
        let transaction = interactor.into_transaction();

        log.apply_or_revert(&mut lookup, transaction)?;

        let server = Self {
            lookup,
            root,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            reservation,
            reaction,
        };

        Ok(server)
    }

    pub fn load_from_save(
        player_init: PlayerInit,
        reaction: Reaction,
        save: Save<ItemCatalog, Root, PlayerInit>,
    ) -> Result<Self, LoadError> 
    where 
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>>,
        ItemCatalog: item::Catalog<Canonical<ItemCatalog>>,
    {
        let Save {
            snapshot,
            next_transaction_id,
            reservation,
            player_manager,
        } = save;

        let reservation = reservation.reservation();

        let mut lookup = Canonical::new();
        
        let root = snapshot.root();
        snapshot.apply(&mut lookup)?;

        let log = log::Server::new_with_next_id(next_transaction_id);

        let server = Self {
            lookup,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            root,
            reservation,
            reaction,
        };

        Ok(server)
    }

    pub fn apply_signal<Interaction>(&mut self, sender: player::Id,  arg: signal::Arg<Interaction>) 
        -> Result<
            Output<ActionCatalog, Reaction::GameOutcome, signal::Ret>, 
            signal::Error
        >
    where
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>> + Clone,
        Interaction: crate::Interaction<ActionCatalog, Root, Output = Reaction::Input>,
        Reaction: interaction::Reaction<ItemCatalog, ActionCatalog, Root, GameOutcome: Clone>,
    {
        let signal::Arg {
            signal,
        } = arg;

        let mut state = State::new();

        match signal {
            signal::Internal::ApplyInteraction(apply_interaction) => 
                self.apply_interaction(&mut state, sender, apply_interaction)?,
        }

        Ok(state.into_output(signal::Ret { }))
    }

    pub fn add_player(&mut self, arg: add_player::Arg<PlayerInit::In>) 
        -> Result<
            Output<ActionCatalog, Reaction::GameOutcome, add_player::Ret<ItemCatalog, ActionCatalog, Root>>, 
            add_player::Error
        > 
    where 
        Reaction: interaction::Reaction<ItemCatalog, ActionCatalog, Root, GameOutcome: Clone>,
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>> + Clone,
        PlayerInit: crate::Init<ItemCatalog, ActionCatalog, Root, Out = ()>,
    {
        let add_player::Arg {
            init_input,
        } = arg;

        let mut state = State::new();

        let mut interactor = Interactor::new(&self.lookup, &self.reservation, self.root);
        self.player_manager.initialize(&mut interactor, init_input)
            .map_err(crate::TempError::discard)?;

        let transaction = interactor.into_transaction();

        self.apply_transaction(&mut state, None, transaction)
            .unwrap();
            // TODO
            //?;
        // let transaction = self.log.apply_or_revert(&mut self.lookup, transaction)?;

        // TODO 640K ought to be enough for anybody
        let (reservation, reservation_range) = self.reservation.split(1024 * 640)
            .expect("depleted id pool");

        let local_player_id = self.player_manager.add(reservation_range);

        // TODO cache snapshots and reuse them if they are recent enough
        let snapshot = self.snapshot()
            .map_err(crate::TempError::discard)?;
        let transactions = Transactions::new(self.log.next_id());

        let client_init = client::init::Arg {
            snapshot,
            transactions,
            reservation,
            local_player_id,
        };

        Ok(state.into_output(add_player::Ret {
            client_init,
            player_id: local_player_id,
        }))
    }

    fn apply_transaction(&mut self, state: &mut State<ActionCatalog, Reaction::GameOutcome>, interaction_transaction: Option<InteractionTransaction<Reaction::Input>>, transaction: Transaction<ActionCatalog>) 
        -> Result<(), signal::Error>
    where
        Reaction: interaction::Reaction<ItemCatalog, ActionCatalog, Root, GameOutcome: Clone>, 
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>> + Clone,
    {
        match self.log.apply_or_revert(&mut self.lookup, transaction) {
            // Desync if revert fails
            Err(log::Error::Revert(revert_error)) => return Err(crate::TempError.into()),
            // Reject if transaction fails
            Err(_) => { },
            Ok(confirmed_transaction) => {
                let mut interactor = Interactor::new(&self.lookup, &self.reservation, self.root);
                let (game_outcome, pending_transaction) = if let Some(interaction_transaction) = interaction_transaction {
                    // TODO instead of desyncing, we could hard undo the transaction (no one else has observed it yet)
                    let game_outcome = self.reaction.apply(&mut interactor, interaction_transaction.interaction_output)
                        .map_err(crate::TempError::discard)?;

                    (game_outcome, Some((interaction_transaction.sender, interaction_transaction.pending_transaction_id)))
                } else {
                    (None, None)
                };

                let transaction = interactor.into_transaction();
                let reaction = self.log.apply_or_revert(&mut self.lookup, transaction)
                    .map_err(crate::TempError::discard)?;

                for player_id in self.player_manager.iter() {
                    let signal: Option<client::signal::Internal<_, _>> = if let Some((owner, pending_transaction_id)) = pending_transaction {
                        // If this transaction originated from a a player interaction, they only need a confirmation (and final tx id),
                        // but all other clients need the whole transaction details
                        if owner == player_id {
                            Some(client::signal::InteractionResult {
                                pending_transaction_id,
                                confirmed_transaction_id: Some(confirmed_transaction.id),
                            }.into())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let signal = if let Some(signal) = signal {
                        signal
                    } else {
                        client::signal::ConfirmedTransaction {
                            confirmed_transaction: confirmed_transaction.clone(),
                        }.into()
                    };
                    
                    state.push_signal(player_id, signal);

                    state.push_signal(player_id, 
                        client::signal::ConfirmedTransaction {
                            confirmed_transaction: reaction.clone(),
                        });

                    if let Some(game_outcome) = game_outcome.as_ref() {
                        state.push_signal(player_id, 
                            client::signal::EndGame {
                                game_outcome: game_outcome.clone(),
                            });
                    }
                }

                if let Some(game_outcome) = game_outcome {
                    state.push_event(event::GameComplete { game_outcome });
                }
            }
        }

        Ok(())
    }

    fn apply_interaction<Interaction>(&mut self, state: &mut State<ActionCatalog, Reaction::GameOutcome>, sender: player::Id, apply_interaction: signal::ApplyInteraction<Interaction>) 
        -> Result<(), crate::TempError>
    where 
        Interaction: crate::Interaction<ActionCatalog, Root>,
        Reaction: interaction::Reaction<ItemCatalog, ActionCatalog, Root, Input = Interaction::Output, GameOutcome: Clone>,
        ActionCatalog: action::Catalog<Canonical<ItemCatalog>> + Clone,
    {
        let signal::ApplyInteraction {
            interaction: interaction::Staged {
                interaction,
                expected_versions,
            },
            pending_transaction_id,
        } = apply_interaction;
        
        let mut interactor = Interactor::new(&self.lookup, &self.reservation, self.root);

        // Interaction  must first succeeed...
        if let Ok(interaction_output) = interaction.apply(&mut interactor, sender) {
            let actual_versions = interactor.expected_versions();
            let transaction = interactor.into_transaction();

            // Auto-reject if versions don't match
            if actual_versions == expected_versions {
                let interaction_transaction = InteractionTransaction {
                    sender,
                    pending_transaction_id,
                    interaction_output,
                };
                
                self.apply_transaction(state, Some(interaction_transaction), transaction)
                    .map_err(crate::TempError::discard)?;

                return Ok(());
            }
        }

        state.push_signal(sender, 
            client::signal::InteractionResult {
                pending_transaction_id,
                confirmed_transaction_id: None,
            });

        Ok(())
    }

    pub fn snapshot(&mut self) -> Result<Snapshot<ItemCatalog, Root>, snapshot::CreateError> {
        Snapshot::new(self.root, &self.lookup)
    }
}

struct InteractionTransaction<InteractionOutput> {
    sender: player::Id,
    pending_transaction_id: transaction::Pending, 
    interaction_output: InteractionOutput,
}

#[must_use]
pub struct ApplyInteraction<ActionCatalog, GameOutcome> {
    pub transaction: transaction::Confirmed<ActionCatalog>,
    pub reaction: transaction::Confirmed<ActionCatalog>,
    pub game_outcome: Option<GameOutcome>,
}
