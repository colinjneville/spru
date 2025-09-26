pub mod add_player;
pub mod event;
use std::marker::PhantomData;

pub use event::Event;
pub mod signal;

use crate::{action, client, error::{RecoverableError, RecoverableResult}, game, interaction, interactor::{self, TakeGameOutcome, TakeTriggers}, item::{self, lookup::Canonical}, log, player, reaction, snapshot, state, transaction::{self, Transactions}, visibility, Interactor, Save, Snapshot, Transaction};

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
        // TODO seq ids not yet implemented
        self.outbound.push((player_id, client::signal::Arg { seq: 0, signal: signal.into() }));
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

pub trait Bounded: Sized {
    type State: crate::State<Canonical<Self::State>, Repr: TryFrom<state::Index>>;
    type Action: crate::Action<Canonical<Self::State>, Undo = Self::Action> + Clone;
    type Root: Clone;
    type PlayerInit: crate::player::Init<State = Self::State, Action = Self::Action, Root = Self::Root>;
    type Interaction: crate::Interaction<Action = Self::Action, Root = Self::Root, Trigger = <Self::Reaction as crate::Reaction>::Trigger>;
    type Reaction: crate::Reaction<State = Self::State, Action = Self::Action, Root = Self::Root, GameOutcome: Clone>;

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

    fn apply_signal(&mut self, sender: player::Id,  arg: signal::Arg<Self::Interaction>) 
        -> signal::Result<Output<Self::Action, <Self::Reaction as crate::Reaction>::GameOutcome, signal::Ret>>;

    fn add_player<'l>(&'l mut self, arg: add_player::Arg<<Self::PlayerInit as crate::player::Init>::In>) 
        -> add_player::Result<Output<Self::Action, <Self::Reaction as crate::Reaction>::GameOutcome, add_player::Ret<Self::State, Self::Action, Self::Root>>>;

    fn create_save(&self) 
        -> Result<Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>, snapshot::CreateError>
    where 
        Self::PlayerInit: Clone,
        Self::Reaction: Clone,
    ;
}

impl<State, Action, Root, PlayerInit, Interaction, Reaction> Bounded for Server<State, Action, Root, PlayerInit, Interaction, Reaction> 
where
    State: crate::State<Canonical<State>, Repr: TryFrom<state::Index>>,
    Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
    Root: Clone,
    PlayerInit: crate::player::Init<State = State, Action = Action, Root = Root>,
    Interaction: crate::Interaction<Action = Action, Root = Root, Trigger = <Reaction as crate::Reaction>::Trigger>,
    Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type PlayerInit = PlayerInit;
    type Interaction = Interaction;
    type Reaction = Reaction;

    fn new<GameInit>(
        game_init: GameInit, 
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> crate::TempResult<Self>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    {
        Self::new(game_init, player_init, reaction)
    }

    fn load_from_save(
        save: Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>,
    ) -> Result<Self, LoadError> {
        Self::load_from_save(save)
    }

    fn apply_signal(&mut self, sender: player::Id,  arg: signal::Arg<Self::Interaction>) 
        -> signal::Result<Output<Self::Action, <Self::Reaction as crate::Reaction>::GameOutcome, signal::Ret>> 
    {
        Self::apply_signal(self, sender, arg)
    }

    fn add_player<'l>(&'l mut self, arg: add_player::Arg<<Self::PlayerInit as crate::player::Init>::In>) 
        -> add_player::Result<Output<Self::Action, <Self::Reaction as crate::Reaction>::GameOutcome, add_player::Ret<Self::State, Self::Action, Self::Root>>> 
    {
        Self::add_player(self, arg)
    }

    fn create_save(&self) 
        -> Result<Save<Self::State, Self::Root, Self::PlayerInit, Self::Reaction>, snapshot::CreateError> 
    where
        PlayerInit: Clone,
        Reaction: Clone,
    {
        Self::create_save(self)
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

impl<State, Action, Root, PlayerInit, Interaction, Reaction> Server<State, Action, Root, PlayerInit, Interaction, Reaction> {
    pub fn new<GameInit>(
        game_init: GameInit, 
        player_init: PlayerInit,
        reaction: Reaction,
    ) -> crate::TempResult<Self> 
    where 
        Action: crate::Action<Canonical<State>, Undo = Action>,
        GameInit: game::Init<State = State, Action = Action, Root = Root>,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>,
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

    pub fn load_from_save(
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
            signal::Error,
        >
    where
        Action: crate::Action<Canonical<State>, Undo = Action> + Clone,
        Interaction: crate::Interaction<Action = Action, Root = Root, Trigger = Reaction::Trigger>,
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
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

    pub fn add_player<'l>(&'l mut self, arg: add_player::Arg<PlayerInit::In>) 
        -> Result<
            Output<Action, Reaction::GameOutcome, add_player::Ret<State, Action, Root>>, 
            add_player::Error
        > 
    where
        // State: crate::State<Canonical<State>>, 
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

    fn build_transaction<Context, Output>(
        &mut self,
        interactor_complete: interactor::Complete<Action, Context, Output>,
    )
        -> RecoverableResult<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), action::Error>
    where
        Reaction: crate::Reaction<State = State, Action = Action, Root = Root>, 
        Action: crate::Action<Canonical<State>, Undo = Action>,
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
        Action: crate::Action<Canonical<State>, Undo = Action>,
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
        let mut interactor = Interactor::new(&mut self.lookup, &self.reservation, context);
        let interaction_error = interaction.apply(&mut interactor).err();
        let result = match interactor.complete(interaction_error) {
            Ok(complete) => {
                // let records = interactor.take_records();
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

    pub fn create_save(&self)
        -> Result<Save<State, Root, PlayerInit, Reaction>, snapshot::CreateError>
    where
        PlayerInit: Clone,
        Reaction: Clone,
        Root: Clone,
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

#[must_use]
#[derive(Debug)]
pub struct ApplyInteraction<Action, GameOutcome> {
    pub transaction: transaction::Confirmed<Action>,
    pub reactions: Vec<transaction::Confirmed<Action>>,
    pub game_outcome: Option<GameOutcome>,
}
