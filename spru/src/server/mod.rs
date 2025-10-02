pub mod add_player;
pub mod event;
use std::marker::PhantomData;

use derive_where::derive_where;
pub use event::Event;
pub mod signal;

use crate::{action, client, error::{RecoverableError, RecoverableResult}, game, interaction, interactor::{self, TakeGameOutcome, TakeTriggers}, item::{self, lookup::Canonical}, log, player, reaction, snapshot, state, transaction::{self, Transactions}, visibility, Interactor, Save, Snapshot, Transaction};

#[derive_where(Debug; client::signal::Arg<Server::Common>, Event<Server>, Ret)]
pub struct Output<Server: self::Server, Ret> {
    pub outbound: Vec<(player::Id, client::signal::Arg<Server::Common>)>,
    pub events: Vec<Event<Server>>,
    pub ret: Ret,
}

#[derive_where(Debug; client::signal::Arg<Server::Common>, Event<Server>)]
struct Messaging<Server: self::Server> {
    pub outbound: Vec<(player::Id, client::signal::Arg<Server::Common>)>,
    pub events: Vec<Event<Server>>,
}

impl<Server: self::Server> Messaging<Server> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<client::signal::Internal<Server::Common>>>(&mut self, player_id: player::Id, signal: S) {
        // TODO seq ids not yet implemented
        self.outbound.push((player_id, client::signal::Arg { seq: 0, signal: signal.into() }));
    }

    pub fn push_event<E: Into<Event<Server>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Server, Ret> {
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

// #[derive(Debug)]
// #[derive(thiserror::Error)]
// pub enum NewError {
//     #[error("{0}")]
//     Lookup(#[from] lookup::Error),
//     #[error(transparent)]
//     Init(game::init::Error),
// }

// impl From<game::init::Error> for NewError {
//     fn from(value: game::init::Error) -> Self {
//         match value {
//             game::init::Error::Lookup(e) => Self::Lookup(e),
//             game::init::Error::Init(e) => Self::Init(e),
//         }
//     }
// }

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Snapshot(#[from] snapshot::ApplyError),
}

// #[derive(Debug)]
// #[derive(thiserror::Error)]
// pub enum ApplyInteractionError {
//     Lookup(#[from] lookup::Error), 
//     Log(#[from] log::Error),
//     Interaction(interaction::Error),
//     /// Interaction uses older versions than on the server
//     Stale,
// }

// impl From<interaction::Error> for ApplyInteractionError {
//     fn from(value: interaction::Error) -> Self {
//         match value {
//             interaction::Error::Lookup(e) => Self::Lookup(e),
//             interaction::Error::Interaction(e) => Self::Interaction(e),
//         }
//     }
// }

// Temporary solution because we need split borrows for `root`
macro_rules! build_transaction {
    ($this:ident, $complete:ident) => {
        Self::build_transaction_internal(
            &mut $this.lookup, 
            &mut $this.log, 
            &$this.reservation, 
            &$this.root, 
            &$this.reaction, 
            $complete,
        )
    }
}

pub trait Server: Sized {
    type State: crate::State;
    type Action: crate::Action<State = Self::State> + Clone;
    type Root: Clone;
    type PlayerInit: crate::player::Init<State = Self::State, Action = Self::Action, Root = Self::Root>;
    type Interaction: crate::Interaction<State = Self::State, Action = Self::Action, Root = Self::Root, Trigger = <Self::Reaction as crate::Reaction>::Trigger> + Clone;
    type Reaction: crate::Reaction<State = Self::State, Action = Self::Action, Root = Self::Root, GameOutcome: Clone>;

    type Common: crate::Common<
        State = Self::State, 
        Action = Self::Action, 
        Root = Self::Root, 
        GameOutcome = <Self::Reaction as crate::Reaction>::GameOutcome, 
        Interaction = Self::Interaction
    >;

    fn new<GameInit>(
        game_init: GameInit, 
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> crate::TempResult<Self>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    ;

    fn load_from_save(
        save: Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>,
    ) -> Result<Self, LoadError>;

    fn apply_signal(&mut self, sender: player::Id,  arg: signal::Arg<Self::Common>) 
        -> signal::Result<Output<Self, signal::Ret>>;

    fn add_player<'l>(&'l mut self, arg: add_player::Arg<Self>) 
        -> add_player::Result<Output<Self, add_player::Ret<Self>>>;

    fn create_save(&self) 
        -> Result<Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>, snapshot::CreateError>
    where 
        Self::PlayerInit: Clone,
        Self::Reaction: Clone,
    ;
}

impl<State, Action, Root, Interaction, Reaction, PlayerInit> Server for Impl<State, Action, Root, Interaction, Reaction, PlayerInit> 
where
    State: crate::State,
    Action: crate::Action<State = State> + Clone,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root, Trigger = <Reaction as crate::Reaction>::Trigger> + Clone,
    Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
    PlayerInit: crate::player::Init<State = State, Action = Action, Root = Root>,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type PlayerInit = PlayerInit;
    type Interaction = Interaction;
    type Reaction = Reaction;

    type Common = crate::common::Impl<
        Self::State, 
        Self::Action, 
        Self::Root, 
        <Self::Reaction as crate::Reaction>::GameOutcome, 
        Self::Interaction
    >;

    fn new<GameInit>(
        game_init: GameInit, 
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> crate::TempResult<Self>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    {
        let player_manager = player::Manager::new(player_init);

        let reservation = item::id::Reservation::all();

        let mut lookup = Canonical::new();

        let context = game::init::Context { };

        // If we fail at any point, there is no need to attempt undo since we scrapping the server anyway
        let mut interactor = Interactor::new(&mut lookup, &reservation, context);
        let root = game_init.initialize(&mut interactor)
            .map_err(crate::TempError::discard)?;
        let interactor_complete = interactor.complete(None::<action::Error>)
            .map_err(crate::TempError::discard)?;

        let mut log = log::Server::new();

        let (_game_init_transaction, _game_outcome) = 
            Self::build_transaction_internal(
                &mut lookup, 
                &mut log, 
                &reservation, 
                &root, 
                &reaction, 
                interactor_complete,
            )
            .map_err(crate::TempError::discard)?;

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

    fn load_from_save(
        save: Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>,
    ) -> Result<Self, LoadError> {
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

    fn apply_signal(&mut self, sender: player::Id,  arg: signal::Arg<Self::Common>) 
        -> signal::Result<Output<Self, signal::Ret>> 
    {
        let signal::Arg {
            seq: _seq,
            signal,
        } = arg;

        let mut messaging = Messaging::new();

        match signal {
            signal::Internal::ApplyInteraction(apply_interaction) => 
                self.apply_interaction(&mut messaging, sender, apply_interaction)?,
        }

        Ok(messaging.into_output(signal::Ret { }))
    }

    fn add_player<'l>(&'l mut self, arg: add_player::Arg<Self>) 
        -> add_player::Result<Output<Self, add_player::Ret<Self>>> 
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
            // To be overwritten in player::Manager::add
            player: player::Id::ZERO,
        };
        let interactor = Interactor::new(&mut self.lookup, &self.reservation, context);

        let result = match self.player_manager.add(interactor, reservation_range, init_input) {
            Ok(complete) => {
                let added_player_id = complete.context.player;
                build_transaction!(self, complete)
                // self.build_transaction(complete)
                    .map(|(transaction, game_outcome)| (transaction, game_outcome, added_player_id))
                    .map_err(RecoverableError::map)
            }
            Err(err) => Err(err),
        };
        
        match result {
            // No reactions are currently permitted on init, so no GameOutcome is possible 
            Ok((transaction, _game_outcome, added_player_id)) => {
                for player_id in self.player_manager.iter() {
                    if player_id != added_player_id {
                        messaging.push_signal(player_id, client::signal::ConfirmedTransaction {
                            confirmed_transaction: transaction.clone(),
                        });
                    }
                }

                // TODO cache snapshots and reuse them if they are recent enough
                let snapshot = self.create_snapshot()
                    .map_err(crate::TempError::discard)?;
                let transactions = Transactions::new(self.log.next_id());

                let client_init = client::init::Arg {
                    snapshot,
                    transactions,
                    reservation,
                    local_player_id: added_player_id,
                };

                Ok(messaging.into_output(add_player::Ret {
                    client_init,
                    player_id: added_player_id,
                }))
            },
            Err(err) => {
                self.player_manager.revert_add();
                // TODO the reservation is lost here
                Err(crate::TempError::discard(err).into())
            }
        }
    }

    fn create_save(&self) 
        -> Result<Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>, snapshot::CreateError> 
    where
        PlayerInit: Clone,
        Reaction: Clone,
    {
        let snapshot = self.create_snapshot()?;
        Ok(Save {
            snapshot,
            next_transaction_id: self.log.next_id(),
            reservation: self.reservation.range(),
            player_manager: self.player_manager.clone(),
            reaction: self.reaction.clone(),
        })
    }
}



#[derive(Debug)]
pub struct Impl<State, Action, Root, Interaction, Reaction, PlayerInit> {
    lookup: item::lookup::Canonical<State>,
    root: Root,
    player_manager: player::Manager<PlayerInit>,
    log: log::Server<Action>,
    visibility: visibility::Manager,
    reservation: item::id::Reservation,
    _interaction: PhantomData<Interaction>,
    reaction: Reaction,
}

impl<State, Action, Root, Interaction, Reaction, PlayerInit> Impl<State, Action, Root, Interaction, Reaction, PlayerInit>
where
    State: crate::State,
    Action: crate::Action<State = State> + Clone,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root, Trigger = <Reaction as crate::Reaction>::Trigger> + Clone,
    Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
    PlayerInit: crate::player::Init<State = State, Action = Action, Root = Root>,
{
    fn build_transaction<Context, Output>(
        &mut self,
        interactor_complete: interactor::Complete<Action, Context, Output>,
    )
        -> RecoverableResult<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), action::Error>
    where
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>, 
        Action: crate::Action<State = State>,
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger> + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        Self::build_transaction_internal(
            &mut self.lookup, 
            &mut self.log, 
            &self.reservation, 
            &self.root, 
            &self.reaction, 
            interactor_complete,
        )
    }

    fn build_transaction_internal<Context, Output>(
        lookup: &mut Canonical<State>,
        log: &mut log::Server<Action>,
        reservation: &item::id::Reservation,
        root: &Root,
        reaction: &Reaction,

        interactor_complete: interactor::Complete<Action, Context, Output>,
    )
        -> RecoverableResult<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), action::Error>
    where
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>, 
        Action: crate::Action<State = State>,
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger> + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        let interactor::Complete {
            expected_versions: _expected_versions,
            do_records: mut all_do_records,
            undo_records: mut all_undo_records,
            context,
            mut output,
        } = interactor_complete;

        let mut reaction_context = reaction::Context {
            root, 
            player: context.player_context(),
        };

        let mut triggers = output.take_triggers();
        let mut game_outcome = None;

        while let Some(trigger) = triggers.pop_front() {
            let mut interactor = Interactor::new(lookup, reservation, reaction_context);
            if let Some(go) = game_outcome.take() {
                interactor.set_game_outcome(go);
            }
            
            let interactor_error = reaction.apply(&mut interactor, trigger).err().map(Into::into);
            match interactor.complete(interactor_error) {
                Ok(complete) => {
                    let crate::interactor::Complete {
                        expected_versions: _expected_versions,
                        do_records,
                        undo_records,
                        context,
                        mut output,
                    } = complete;

                    all_do_records.extend(do_records);
                    all_undo_records.extend(undo_records);

                    triggers.append(&mut output.take_triggers());

                    game_outcome = output.take_game_outcome();

                    reaction_context = context;
                },
                Err(mut err) => {
                    // Only bother with undo if we haven't hit an unrecoverable error already
                    if err.is_recovered() {

                        if let Err(undo_error) = all_undo_records.apply(lookup) {
                            // Undo failed, state is inconsistent
                            err.recovery_error = Some(undo_error);
                        }
                    }

                    return Err(err);
                }
            }
        }

        let undo_transaction = Transaction::new(all_undo_records);
        let transaction_id = log.register_undo(undo_transaction);

        let do_transaction = transaction::Confirmed::new(transaction_id, Transaction::new(all_do_records));
        
        Ok((do_transaction, game_outcome))
    }

    fn apply_interaction(&mut self, messaging: &mut Messaging<Self>, sender: player::Id, apply_interaction: signal::ApplyInteraction<<Self as Server>::Common>) 
        -> Result<(), crate::TempError>
    {
        let signal::ApplyInteraction {
            interaction: interaction::Staged {
                interaction,
                expected_versions,
            },
            pending_transaction_id,
        } = apply_interaction;
        
        let context = interaction::Context::new(&self.root, sender);
        let mut interactor = Interactor::new(&mut self.lookup, &self.reservation, context);
        let interaction_error = interaction.apply(&mut interactor).err();
        let result = match interactor.complete(interaction_error) {
            Ok(complete) => {
                let interaction_record_count = complete.do_records.len();

                // Auto-reject if versions don't match
                if complete.expected_versions == expected_versions {
                    
                    // match self.build_transaction(complete) {
                    match build_transaction!(self, complete) {
                        Ok((confirmed_transaction, game_outcome)) => {
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

                            Ok(())
                        }
                        Err(err) => Err(crate::TempError::discard::<RecoverableError<action::Error>>(err)),
                    }
                } else {
                    // Invalid versions
                    Err(crate::TempError::new())
                }
            },
            Err(err) => Err(crate::TempError::discard(err)),
        };

        match result {
            Ok(()) => Ok(()),
            Err(err) => {
                // Something failed, reject the interaction
                messaging.push_signal(sender, 
                    client::signal::InteractionResult {
                        pending_transaction_id,
                        confirmed_transaction_id: None,
                    });

                Err(err)
            }
        }
    }

    fn create_snapshot(&self) -> Result<Snapshot<State, Root>, snapshot::CreateError> 
    where 
        Root: Clone,
    {
        Snapshot::new(self.root.clone(), &self.lookup)
    }
}

#[must_use]
#[derive(Debug)]
pub struct ApplyInteraction<Action, GameOutcome> {
    pub transaction: transaction::Confirmed<Action>,
    pub reactions: Vec<transaction::Confirmed<Action>>,
    pub game_outcome: Option<GameOutcome>,
}
