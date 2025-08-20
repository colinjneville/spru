pub mod add_player;
pub mod event;
use std::{collections::VecDeque, marker::PhantomData};

pub use event::Event;
pub mod signal;

use crate::{client, game, interaction, item::{self, lookup::{canonical, Canonical}}, log, player, reaction, record::{self, Records}, snapshot, state, transaction::{self, Transactions}, visibility, Interactor, Save, Snapshot, Transaction};

#[derive(Debug)]
pub struct Output<Action, GameOutcome, Ret> {
    pub outbound: Vec<(player::Id, client::signal::Arg<Action, GameOutcome>)>,
    pub events: Vec<Event<GameOutcome>>,
    pub ret: Ret,
}

#[derive(Debug)]
struct Messaging<Action, GameOutcome> {
    pub outbound: Vec<(player::Id, client::signal::Arg<Action, GameOutcome>)>,
    pub events: Vec<Event<GameOutcome>>,
}

impl<Action, GameOutcome> Messaging<Action, GameOutcome> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<client::signal::Internal<Action, GameOutcome>>>(&mut self, player_id: player::Id, signal: S) {
        self.outbound.push((player_id, client::signal::Arg { signal: signal.into() }));
    }

    pub fn push_event<E: Into<Event<GameOutcome>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Action, GameOutcome, Ret> {
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
pub enum NewError<GameInitError, ActionError> {
    #[error(transparent)]
    Lookup(#[from] canonical::Error),
    #[error(transparent)]
    Init(GameInitError),
    #[error(transparent)]
    Log(#[from] log::Error<canonical::Error, ActionError>)
}

impl<GameInitError, ActionError> From<game::init::Error<GameInitError>> for NewError<GameInitError, ActionError> {
    fn from(value: game::init::Error<GameInitError>) -> Self {
        match value {
            game::init::Error::Lookup(e) => Self::Lookup(e),
            game::init::Error::Init(e) => Self::Init(e),
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
pub enum AddPlayerError<PlayerInitError, ActionError> {
    #[error(transparent)]
    Snapshot(#[from] snapshot::CreateError),
    #[error(transparent)]
    Lookup(#[from] canonical::Error), 
    #[error(transparent)]
    Init(PlayerInitError),
    #[error(transparent)]
    Log(#[from] log::Error<canonical::Error, ActionError>),
}

impl<PlayerInitError, ActionError> From<player::manager::InitializeError<PlayerInitError>> for AddPlayerError<PlayerInitError, ActionError> {
    fn from(value: player::manager::InitializeError<PlayerInitError>) -> Self {
        match value {
            player::manager::InitializeError::Lookup(e) => Self::Lookup(e),
            player::manager::InitializeError::Init(e) => Self::Init(e),
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ApplyInteractionError<ActionError, InteractionError> {
    Lookup(#[from] canonical::Error), 
    Log(#[from] log::Error<canonical::Error, ActionError>),
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

#[derive(Debug)]
pub struct Server<State, Action, Root, PlayerInit, Interaction, Reaction> {
    lookup: item::lookup::Canonical<State>,
    root: Root,
    player_manager: player::Manager<PlayerInit>,
    log: log::Server<Action>,
    visibility: visibility::Manager,
    reservation: item::id::Reservation,
    _interaction: PhantomData<Interaction>,
    reaction: Reaction,
}

impl<State, Action, Root, PlayerInit, Interaction, Reaction> Server<State, Action, Root, PlayerInit, Interaction, Reaction> {
    pub fn new<GameInit>(
        game_init: GameInit, 
        player_init: PlayerInit,
        reaction: Reaction,
    ) -> Result<Self, NewError<GameInit::Error, Action::Error>> 
    where 
        Action: crate::Action<Canonical<State>, Undo = Action>,
        State: crate::State<Canonical<State>>,
        GameInit: game::Init<State = State, Action = Action, Root = Root>,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>,
    {
        let player_manager = player::Manager::new(player_init);

        let reservation = item::id::Reservation::all();

        let mut lookup = Canonical::new();

        let context = game::init::Context { };
        let mut interactor = Interactor::new(&lookup, &reservation, context);
        let root = game_init.initialize(&mut interactor)?;

        let mut log = log::Server::new();
        let records = interactor.take_records();

        let _game_init_transaction = Self::build_transaction_internal(
            &mut lookup, 
            &mut log, 
            &reservation, 
            &root, 
            &reaction, 
            records, 
            None, 
            VecDeque::new(),
        )?;

        let server = Self {
            lookup,
            root,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            reservation,
            _interaction: Default::default(),
            reaction,
        };

        Ok(server)
    }

    pub fn load_from_save(
        player_init: PlayerInit,
        save: Save<State, Root, PlayerInit, Reaction>,
    ) -> Result<Self, LoadError> 
    where 
        State: crate::State<Canonical<State>, Repr: TryFrom<state::Index>>,
        Action: crate::Action<Canonical<State>>,
        Root: Clone,
    {
        let Save {
            snapshot,
            next_transaction_id,
            reservation,
            player_manager,
            reaction,
        } = save;

        let reservation = reservation.reservation();

        let mut lookup = Canonical::new();
        
        let root = snapshot.root()
            .clone();
        snapshot.apply(&mut lookup)?;

        let log = log::Server::new_with_next_id(next_transaction_id);

        let server = Self {
            lookup,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            root,
            reservation,
            _interaction: PhantomData,
            reaction,
        };

        Ok(server)
    }

    pub fn apply_signal(&mut self, sender: player::Id,  arg: signal::Arg<Interaction>) 
        -> Result<
            Output<Action, Reaction::GameOutcome, signal::Ret>, 
            signal::Error<Action::Error>,
        >
    where
        Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
        Interaction: crate::Interaction<Action = Action, Root = Root, Trigger = Reaction::Trigger>,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
    {
        let signal::Arg {
            signal,
        } = arg;

        let mut messaging = Messaging::new();

        match signal {
            signal::Internal::ApplyInteraction(apply_interaction) => 
                self.apply_interaction(&mut messaging, sender, apply_interaction)?,
        }

        Ok(messaging.into_output(signal::Ret { }))
    }

    pub fn add_player<'l>(&'l mut self, arg: add_player::Arg<PlayerInit::In>) 
        -> Result<
            Output<Action, Reaction::GameOutcome, add_player::Ret<State, Action, Root>>, 
            add_player::Error
        > 
    where
        State: crate::State<Canonical<State>>, 
        Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
        PlayerInit: player::Init<State = State, Action = Action, Root = Root>,
        Root: Clone,
    {
        let add_player::Arg {
            init_input,
        } = arg;

        let mut messaging = Messaging::new();

        // self.apply_transaction(&mut messaging, None, transaction)
        //     .unwrap();
        //     // TODO
        //     //?;
        // let transaction = self.log.apply_or_revert(&mut self.lookup, transaction)?;

        // TODO 640K ought to be enough for anybody
        let (reservation, reservation_range) = self.reservation.split(1024 * 640)
            .expect("depleted id pool");

        let context = player::init::Context {
            root: &self.root,
            // To be overwritten in player::Manager
            player: player::Id::ZERO,
        };
        let mut interactor = Interactor::new(&self.lookup, &self.reservation, context);

        let local_player_id = self.player_manager.add(&mut interactor, reservation_range, init_input)
            .map_err(crate::TempError::discard)?;

        let records = interactor.take_records();
        
        match self.build_transaction(records, Some(local_player_id), VecDeque::new()) {
            // No reactions are currently permitted on init, so no GameOutcome is possible 
            Ok((transaction, _game_outcome)) => {
                for player_id in self.player_manager.iter() {
                    if player_id != local_player_id {
                        messaging.push_signal(player_id, client::signal::ConfirmedTransaction {
                            confirmed_transaction: transaction.clone(),
                        });
                    }
                }

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

                Ok(messaging.into_output(add_player::Ret {
                    client_init,
                    player_id: local_player_id,
                }))
            },
            Err(err) => {
                self.player_manager.revert_add();
                // TODO the reservation is lost here
                Err(crate::TempError::discard(err).into())
            }
        }
    }

    fn build_transaction(
        &mut self,
        initial_records: Records<Action>,
        player_context: Option<player::Id>,
        triggers: VecDeque<Reaction::Trigger>,
    )
        -> Result<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), log::Error<canonical::Error, Action::Error>>
    where
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>, 
        Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
    {
        Self::build_transaction_internal(
            &mut self.lookup, 
            &mut self.log, 
            &self.reservation, 
            &self.root, 
            &self.reaction, 
            initial_records, 
            player_context, 
            triggers
        )
    }

    fn build_transaction_internal(
        lookup: &mut Canonical<State>,
        log: &mut log::Server<Action>,
        reservation: &item::id::Reservation,
        root: &Root,
        reaction: &Reaction,

        initial_records: Records<Action>,
        player_context: Option<player::Id>,
        triggers: VecDeque<Reaction::Trigger>,
    )
        -> Result<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), log::Error<canonical::Error, Action::Error>>
    where
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>, 
        Action: crate::Action<Canonical<State>, Undo = Action>,
    {
        let mut undo_records = initial_records.apply_or_revert(lookup)?;
        let mut do_records = initial_records;
        let mut reaction_context = reaction::Context::new(root, player_context, triggers);
        
        let result = 'result: {
            while let Some(trigger) = reaction_context.dequeue_trigger() {
                let mut interactor = Interactor::new(lookup, reservation, reaction_context);
                match reaction.apply(&mut interactor, trigger) {
                    Ok(()) => {
                        let reaction_do_records = interactor.take_records();
                        reaction_context = interactor.into_context();

                        match reaction_do_records.apply_or_revert(lookup) {
                            Ok(reaction_undo_records) => {
                                do_records.extend(reaction_do_records);
                                undo_records.extend(reaction_undo_records);
                            }
                            Err(err) => break 'result Err(err),
                        }
                    }
                    Err(err) => break 'result Err(log::Error::Record(record::Error::Lookup(err))),
                }
            }
            Ok(reaction_context.take_game_outcome())
        };

        match result {
            Ok(game_outcome) => { 
                let transaction_id = log.register_undo(Transaction::new(undo_records));
                Ok((transaction::Confirmed::new(transaction_id, Transaction::new(do_records)), game_outcome))
            }
            // Unrecoverable error
            Err(err @ log::Error::Revert(_)) => {
                Err(err)
            }
            // Recoverable error, attempt undo
            Err(log::Error::Record(initial_error)) => {
                match undo_records.apply(lookup) {
                    // Undo succeeded, return a recoverable error
                    Ok(_) => Err(log::Error::Record(initial_error)),
                    // Undo failed, state is inconsistent
                    Err(undo_error) => 
                        Err(log::Error::Revert(log::RevertError { initial: Some(initial_error), fatal: undo_error }))
                }
            }
        }
    }

    // fn apply_transaction2(
    //     &mut self, 
    //     messaging: &mut Messaging<Action, Reaction::GameOutcome>, 
    //     interaction_transaction: Option<InteractionTransaction<Interaction::Trigger>>, 
    //     transaction: Transaction<Action>,
    //     pending_transaction_id: Option<transaction::Pending>,
    //     reaction_context: reaction::Context<Root, Reaction::Trigger, Reaction::GameOutcome>,
        
    // ) 
    //     -> Result<(), signal::Error>
    // where
    //     Interaction: crate::Interaction<Action = Action, Root = Root, Trigger = Reaction::Trigger>,
    //     Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>, 
    //     Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
    // {
    //     match self.log.apply_or_revert(&mut self.lookup, transaction) {
    //         // Desync if revert fails
    //         Err(log::Error::Revert(revert_error)) => return Err(crate::TempError::discard(revert_error).into()),
    //         // Reject if transaction fails
    //         Err(_) => { },
    //         Ok(confirmed_transaction) => {
    //             let context = reaction::Context::new(&self.root, Some(player_context),);
    //             let mut interactor = Interactor::new(&self.lookup, &self.reservation, context);

    //             let (game_outcome, pending_transaction) = if let Some(interaction_transaction) = interaction_transaction {
    //                 // TODO instead of desyncing, we could hard undo the transaction (no one else has observed it yet)
    //                 self.reaction.apply(&mut interactor, interaction_transaction.interaction_output)
    //                     .map_err(crate::TempError::discard)?;

    //                 (game_outcome, Some((interaction_transaction.sender, interaction_transaction.pending_transaction_id)))
    //             } else {
    //                 (None, None)
    //             };

    //             let transaction = interactor.take_transaction();
    //             let reaction = self.log.apply_or_revert(&mut self.lookup, transaction)
    //                 .map_err(crate::TempError::discard)?;

    //             for player_id in self.player_manager.iter() {
    //                 let signal: Option<client::signal::Internal<_, _>> = if let Some((owner, pending_transaction_id)) = pending_transaction {
    //                     // If this transaction originated from a a player interaction, they only need a confirmation (and final tx id),
    //                     // but all other clients need the whole transaction details
    //                     if owner == player_id {
    //                         Some(client::signal::InteractionResult {
    //                             pending_transaction_id,
    //                             confirmed_transaction_id: Some(confirmed_transaction.id),
    //                         }.into())
    //                     } else {
    //                         None
    //                     }
    //                 } else {
    //                     None
    //                 };

    //                 let signal = if let Some(signal) = signal {
    //                     signal
    //                 } else {
    //                     client::signal::ConfirmedTransaction {
    //                         confirmed_transaction: confirmed_transaction.clone(),
    //                     }.into()
    //                 };
                    
    //                 messaging.push_signal(player_id, signal);

    //                 messaging.push_signal(player_id, 
    //                     client::signal::ConfirmedTransaction {
    //                         confirmed_transaction: reaction.clone(),
    //                     });

    //                 if let Some(game_outcome) = game_outcome.as_ref() {
    //                     messaging.push_signal(player_id, 
    //                         client::signal::EndGame {
    //                             game_outcome: game_outcome.clone(),
    //                         });
    //                 }
    //             }

    //             if let Some(game_outcome) = game_outcome {
    //                 messaging.push_event(event::GameComplete { game_outcome });
    //             }
    //         }
    //     }

    //     Ok(())
    // }

    fn apply_interaction(&mut self, messaging: &mut Messaging<Action, Reaction::GameOutcome>, sender: player::Id, apply_interaction: signal::ApplyInteraction<Interaction>) 
        -> Result<(), crate::TempError>
    where 
        Interaction: crate::Interaction<Action = Action, Root = Root, Trigger = Reaction::Trigger>,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
        Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
    {
        let signal::ApplyInteraction {
            interaction: interaction::Staged {
                interaction,
                expected_versions,
            },
            pending_transaction_id,
        } = apply_interaction;
        
        let context = interaction::Context::new(&self.root, sender);
        let mut interactor = Interactor::new(&self.lookup, &self.reservation, context);

        // Interaction must first succeeed...
        if let Ok(()) = interaction.apply(&mut interactor) {
            let actual_versions = interactor.expected_versions();
            let records = interactor.take_records();
            let interaction_record_count = records.len();

            // Auto-reject if versions don't match
            if actual_versions == expected_versions {                
                let triggers = interactor.into_context().into_triggers();
                if let Ok((confirmed_transaction, game_outcome)) = self.build_transaction(records, Some(sender), triggers) {
                    // Everything succeeded, exit before the failure signal below

                    // Send the whole transaction to all other players
                    for player_id in self.player_manager.iter() {
                        if player_id != sender {
                            messaging.push_signal(player_id, client::signal::ConfirmedTransaction {
                                confirmed_transaction: confirmed_transaction.clone(),
                            });
                        }
                    }

                    // Confirm the tentative transaction with the sender + the log generated by reactions
                    let confirmed_transaction_id = confirmed_transaction.id;
                    let mut records = confirmed_transaction.transaction.into_records();
                    records.trim_start(interaction_record_count);

                    messaging.push_signal(sender, 
                    client::signal::InteractionResult {
                        pending_transaction_id,
                        confirmed_transaction_id: Some((confirmed_transaction_id, records)),
                    });

                    if let Some(game_outcome) = game_outcome {
                        for player_id in self.player_manager.iter() {
                            messaging.push_signal(player_id, client::signal::EndGame {
                                game_outcome: game_outcome.clone(),
                            })
                        }

                        messaging.push_event(event::GameComplete {
                            game_outcome,
                        });
                    }

                    return Ok(());
                }
            }
        }

        // Something failed, reject the interaction
        messaging.push_signal(sender, 
            client::signal::InteractionResult {
                pending_transaction_id,
                confirmed_transaction_id: None,
            });

        Ok(())
    }

    pub fn snapshot(&mut self) -> Result<Snapshot<State, Root>, snapshot::CreateError> 
    where 
        Root: Clone,
    {
        Snapshot::new(self.root.clone(), &self.lookup)
    }
}

#[must_use]
pub struct ApplyInteraction<Action, GameOutcome> {
    pub transaction: transaction::Confirmed<Action>,
    pub reactions: Vec<transaction::Confirmed<Action>>,
    pub game_outcome: Option<GameOutcome>,
}
