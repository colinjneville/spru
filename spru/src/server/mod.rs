pub mod add_player;
pub mod save;
pub use save::Save;
pub mod init;
pub mod load;
pub mod manual_trigger;
pub mod event;
use std::{collections::VecDeque, marker::PhantomData};

use derive_where::derive_where;
pub use event::Event;
pub mod signal;
pub use signal::Signal;
use tracing::instrument;

use crate::{action, client, common, error::{RecoverableError, RecoverableResult}, game, interaction, interactor::{self, TakeGameOutcome, TakeTriggers}, item::{self, lookup::Canonical}, log, player, reaction, transaction::{self, Transactions}, visibility, Interactor, Transaction};

#[derive_where(Debug; client::signal::Signal<Server::Common>, Event<Server>, Ret)]
pub struct Output<Server: self::Server, Ret> {
    pub outbound: Vec<(player::Id, client::signal::Signal<Server::Common>)>,
    pub events: Vec<Event<Server>>,
    pub ret: Ret,
}

#[derive_where(Debug; client::signal::Signal<Server::Common>, Event<Server>)]
struct Messaging<Server: self::Server> {
    pub outbound: Vec<(player::Id, client::signal::Signal<Server::Common>)>,
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
        self.outbound.push((player_id, client::signal::Signal { seq: 0, signal: signal.into() }));
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

    fn init<GameInit>(
        game_init: GameInit, 
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> init::Result<Self>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    ;

    fn load(
        save: Save<Self>,
    ) -> load::Result<Self>;

    fn apply_signal(&mut self, sender: player::Id, signal: signal::Signal<Self::Common>) 
        -> signal::Result<Self>;

    fn manual_trigger(&mut self, trigger: <Self::Reaction as crate::Reaction>::Trigger) 
        -> manual_trigger::Result<Self>;

    fn add_player(&mut self, init_input: <Self::PlayerInit as crate::player::Init>::In) 
        -> add_player::Result<Self>;

    fn save(&self) 
        -> save::Result<Self>
    where 
        Self::PlayerInit: Clone,
        Self::Reaction: Clone,
    ;

    fn game_id(&self) -> game::Id;
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

    #[instrument(err, skip_all)]
    fn init<GameInit>(
        game_init: GameInit, 
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> init::Result<Self>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    {
        let player_manager = player::Manager::new(player_init);

        let reservation = item::id::Reservation::all();

        let mut lookup = Canonical::new();

        let context = game::init::Context { };

        let game_init_context = game::init::Error::prepare_context(&game_init);

        // If we fail at any point, there is no need to attempt undo since we scrapping the server anyway
        let mut interactor = Interactor::new(&mut lookup, &reservation, context);
        let root = game_init.initialize(&mut interactor)
            .map_err(game_init_context)
            .map_err(crate::TempError::discard)?;
        let interactor_complete = interactor.complete(None::<action::Error>)
            .map_err(crate::TempError::discard)?;

        let log = log::Server::new();

        let mut inner = ImplInner {
            game_id: game::Id::new(),
            lookup,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            reservation,
            reaction,
            _interaction: PhantomData,
            _root: PhantomData,
        };

        let mut messaging = Messaging::new();

        inner.build_transaction(
            &root, 
            &mut messaging,
            None,
            interactor_complete,
        )
            .map_err(crate::TempError::discard)?;

        let server = Self {
            root,
            inner,
        };

        Ok(server)
    }

    #[instrument(err, skip_all)]
    fn load(
        save: Save<Self>,
    ) -> load::Result<Self> {
        let Save {
            game_id,
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
            root,
            inner: ImplInner {
                game_id,
                lookup,
                player_manager,
                visibility: visibility::Manager::new(),
                log,
                reservation,
                reaction,
                _interaction: PhantomData,
                _root: PhantomData,
            }
        };

        let ret = load::Ret { 
            server, 
            client_inits: todo!(),
        };

        Ok(ret)
    }

    #[instrument(err, skip_all, fields(sender = sender.into_u32()))]
    fn apply_signal(&mut self, sender: player::Id,  arg: signal::Signal<Self::Common>) 
        -> signal::Result<Self> 
    {
        let signal::Signal {
            seq: _seq,
            signal,
        } = arg;

        let mut messaging = Messaging::new();

        match signal {
            signal::Internal::ApplyInteraction(apply_interaction) => {
                self.inner.apply_interaction(&self.root, &mut messaging, sender, apply_interaction)
                    .map_err(signal::Error::Fatal)?;
            }
        }

        Ok(messaging.into_output(signal::Ret { }))
    }

    #[instrument(err, skip_all)]
    fn manual_trigger(&mut self, trigger: <Self::Reaction as crate::Reaction>::Trigger) 
        -> manual_trigger::Result<Self>
    {
        
        struct ManualTriggerContext;

        impl interactor::PlayerContext for ManualTriggerContext {
            fn player_context(&self) -> Option<player::Id> {
                None
            }
        }

        struct ManualTriggerOutput<Trigger>(Option<Trigger>);

        impl<Trigger, GameOutcome> TakeGameOutcome<GameOutcome> for ManualTriggerOutput<Trigger> {
            fn take_game_outcome(&mut self) -> Option<GameOutcome> {
                None
            }
        }

        impl<Trigger> TakeTriggers<Trigger> for ManualTriggerOutput<Trigger> {
            fn take_triggers(&mut self) -> VecDeque<Trigger> {
                let mut v = VecDeque::new();
                v.extend(self.0.take());
                v
            }
        }

        let mut messaging = Messaging::new();

        let complete = interactor::Complete {
            expected_versions: item::version::Expected::new(std::iter::empty()),
            do_records: Default::default(),
            undo_records: Default::default(),
            context: ManualTriggerContext,
            output: ManualTriggerOutput(Some(trigger)),
        };

        self.inner.build_transaction(&self.root, &mut messaging, None, complete)
            .map_err(crate::TempError::discard)?;

        Ok(messaging.into_output(manual_trigger::Ret { }))
    }

    #[instrument(err, skip_all, fields(player_id = tracing::field::Empty))]
    fn add_player<'l>(&'l mut self, init_input: <Self::PlayerInit as crate::player::Init>::In) 
        -> add_player::Result<Self> 
    {
        let mut messaging = Messaging::new();

        // self.apply_transaction(&mut messaging, None, transaction)
        //     .unwrap();
        //     // TODO
        //     //?;
        // let transaction = self.log.apply_or_revert(&mut self.lookup, transaction)?;

        // TODO 640K ought to be enough for anybody
        let (reservation, reservation_range) = self.inner.reservation.split(1024 * 640)
            .expect("depleted id pool");

        let context = player::init::Context {
            root: &self.root,
            // To be overwritten in player::Manager::add
            player: player::Id::ZERO,
        };
        let interactor = Interactor::new(&mut self.inner.lookup, &self.inner.reservation, context);

        let result = match self.inner.player_manager.add(interactor, reservation_range, init_input) {
            Ok(complete) => {
                let added_player_id = complete.context.player;
                tracing::Span::current().record("player_id", &added_player_id.into_u32());

                self.inner.build_transaction(&self.root, &mut messaging, None, complete)
                    .map(|transaction| (transaction, added_player_id))
                    .map_err(RecoverableError::map)
            }
            Err(err) => Err(err),
        };
        
        match result {
            // No reactions are currently permitted on init, so no GameOutcome is possible 
            Ok((transaction, added_player_id)) => {
                let game_id = self.inner.game_id;
                // TODO cache snapshots and reuse them if they are recent enough
                let snapshot = self.inner.create_snapshot(&self.root)
                    .map_err(crate::TempError::discard)?;
                let transactions = Transactions::new(self.inner.log.next_id());

                let client_init = client::init::Arg {
                    game_id,
                    local_player_id: added_player_id,
                    snapshot,
                    transactions,
                    reservation,
                };

                Ok(messaging.into_output(add_player::Ret {
                    client_init,
                }))
            },
            Err(err) => {
                self.inner.player_manager.revert_add();
                // TODO the reservation is lost here
                Err(crate::TempError::discard(err).into())
            }
        }
    }

    #[instrument(err, skip_all)]
    fn save(&self) 
        -> save::Result<Self>
    where
        PlayerInit: Clone,
        Reaction: Clone,
    {
        let snapshot = self.inner.create_snapshot(&self.root)?;
        Ok(Save {
            game_id: self.inner.game_id,
            snapshot,
            next_transaction_id: self.inner.log.next_id(),
            reservation: self.inner.reservation.range(),
            player_manager: self.inner.player_manager.clone(),
            reaction: self.inner.reaction.clone(),
        })
    }

    fn game_id(&self)
        -> game::Id
    {
        self.inner.game_id
    }
}



#[derive(Debug)]
pub struct Impl<State, Action, Root, Interaction, Reaction, PlayerInit> {
    inner: ImplInner<State, Action, Root, Interaction, Reaction, PlayerInit>,
    root: Root,
}

// Needed for split borrows on root
#[derive(Debug)]
struct ImplInner<State, Action, Root, Interaction, Reaction, PlayerInit> {
    game_id: game::Id,
    lookup: item::lookup::Canonical<State>,
    player_manager: player::Manager<PlayerInit>,
    log: log::Server<Action>,
    visibility: visibility::Manager,
    reservation: item::id::Reservation,
    reaction: Reaction,
    _interaction: PhantomData<Interaction>,
    _root: PhantomData<Root>,
}

impl<State, Action, Root, Interaction, Reaction, PlayerInit> ImplInner<State, Action, Root, Interaction, Reaction, PlayerInit> 
where
    State: crate::State,
    Action: crate::Action<State = State> + Clone,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root, Trigger = <Reaction as crate::Reaction>::Trigger> + Clone,
    Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
    PlayerInit: crate::player::Init<State = State, Action = Action, Root = Root>,
{
    #[instrument(err, skip_all)]
    fn apply_interaction(
        &mut self, 
        root: &Root, 
        messaging: &mut Messaging<Impl<State, Action, Root, Interaction, Reaction, PlayerInit>>, 
        sender: player::Id, 
        apply_interaction: signal::ApplyInteraction<<Impl<State, Action, Root, Interaction, Reaction, PlayerInit> as Server>::Common>,
    ) 
        -> action::Result<()>
    {
        let signal::ApplyInteraction {
            interaction: interaction::Staged {
                interaction,
                expected_versions,
                pending_transaction_id,
            },
        } = apply_interaction;
        
        let result = (|| {
            let context = interaction::Context::new(root, sender);
            let mut interactor = Interactor::new(&mut self.lookup, &self.reservation, context);
            let interaction_error = interaction.apply(&mut interactor)
                .map_err(|e| e.with_context(&interaction))
                .err();
            let complete = interactor.complete(interaction_error)?;
            // Auto-reject if versions don't match
            match expected_versions.diff(&complete.expected_versions) {
                Ok(()) => {
                    self.build_transaction(root, messaging, Some(pending_transaction_id), complete)
                        .map_err(RecoverableError::map)
                }
                Err(err) => {
                    tracing::event!(name: "interaction_version_conflict", tracing::Level::INFO, { });

                    Err(RecoverableError::<interaction::Error>::new(err.into()))
                }
            }
        })();

        match result {
            Ok(transaction) => {
                tracing::info!(name: "confirmed_interaction", id = transaction.id.into_u32());
                Ok(())
            }
            Err(err) => {
                tracing::info!(name: "rejected_interaction", error = %err);

                // Something failed, reject the interaction
                messaging.push_signal(sender, 
                    client::signal::InteractionResult {
                        pending_transaction_id,
                        confirmed_transaction_id: None,
                    });

                // First-level errors are expected here, so only report unrecoverable errors
                match err.recovery_error {
                    Some(err) => Err(err),
                    None => Ok(()),
                }
            }
        }
    }

    #[instrument(err, skip_all)]
    fn build_transaction<Context, Output>(
        &mut self, 
        root: &Root,
        messaging: &mut Messaging<Impl<State, Action, Root, Interaction, Reaction, PlayerInit>>, 
        // This is a bit of a kludge to special case Interactions where a Client already has some of the transaction
        // applied locally and only needs log generated from Server Reactions
        pending_transaction: Option<transaction::Pending>, 
        complete: interactor::Complete<Action, Context, Output>
    )
        -> Result<transaction::Confirmed<Action>, RecoverableError<action::Error>>
    where
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger> + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        
        // The player context appears in `add_player` and `Interaction`s. In either case,
        // this player does not need the full transaction. On `add_player`, the added player
        // will receive a snapshot and any log needed to bring it up to date, so we provide nothing
        // here. For an `Interaction`, the interacting player already has the log generated by
        // the interaction itself, but it needs anything generated by Server Reactions.
        // This case is tied to the `pending_transaction` parameter, and the unneeded log is determined
        // by the existing log at this point (`initial_record_count`).
        let player_context = complete.context.player_context();

        let initial_record_count = complete.do_records.len();

        match self.run_reactions(root, complete) {
            Ok((confirmed_transaction, game_outcome)) => {
                // Everything succeeded, exit before the failure signal below

                // Send the whole transaction to all other players
                for player_id in self.player_manager.iter() {
                    if Some(player_id) != player_context {
                        // TODO We should discard empty transactions so 'checking' Reactions don't generate
                        // unnecessary spam. We need to be sure culled transactions have no side effects,
                        // such as GameOutcome, however
                        messaging.push_signal(player_id, client::signal::ConfirmedTransaction {
                            confirmed_transaction: confirmed_transaction.clone(),
                        });
                    }
                }

                if let Some(pending_transaction_id) = pending_transaction {
                    let Some(player_context) = player_context else {
                        unreachable!("Interactions must have a player context");
                    };

                    // Confirm the tentative transaction with the sender + the log generated by reactions
                    let confirmed_transaction_id = confirmed_transaction.id;
                    let mut records = confirmed_transaction.transaction.clone().into_records();
                    records.trim_start(initial_record_count);

                    messaging.push_signal(player_context, 
                        client::signal::InteractionResult {
                            pending_transaction_id,
                            confirmed_transaction_id: Some((confirmed_transaction_id, records)),
                        });
                }

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

                Ok(confirmed_transaction)
            }
            Err(err) => Err(err),
        }
    }

    #[instrument(err, skip_all)]
    fn run_reactions<Context, Output>(
        &mut self,
        root: &Root,
        interactor_complete: interactor::Complete<Action, Context, Output>,
    )
        -> RecoverableResult<(transaction::Confirmed<Action>, Option<Reaction::GameOutcome>), action::Error>
    where
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger> + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        let Self {
            game_id: _id,
            lookup,
            player_manager: _player_manager,
            log,
            visibility: _visibility_manager,
            reservation,
            reaction,
            _interaction,
            _root,
        } = self;

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
        let transaction_len = undo_transaction.records().len();

        let transaction_id = log.register_undo(undo_transaction);

        tracing::event!(name: "transaction", tracing::Level::DEBUG, id = transaction_id.get(), len = transaction_len);

        let do_transaction = transaction::Confirmed::new(transaction_id, Transaction::new(all_do_records));
        
        Ok((do_transaction, game_outcome))
    }

    fn create_snapshot(&self, root: &Root) -> Result<common::Snapshot<State, Root>, common::error::Save> 
    where 
        Root: Clone,
    {
        common::Snapshot::new(root.clone(), &self.lookup)
    }
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
    
    
}
