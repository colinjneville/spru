pub mod error;
mod save;
pub use save::Save;
mod log;
use log::Log;
pub mod event;
use std::{collections::VecDeque, marker::PhantomData};

use derive_where::derive_where;
pub use event::Event;
use tracing::instrument;

use crate::{
    Interactor, Transaction, action,
    common::{
        self,
        error::{PseudoError, RecoverableError},
    },
    game, interaction,
    interactor::{self, TakeGameOutcome, TakeTriggers},
    item::{self, storage::Canonical},
    player, reaction,
    transaction::{self, Transactions},
    visibility,
};

/// Outputs of [Server] functions.
/// Notably, you are responsible for delivering all signals in [Output::outbound] to
/// the appropriate [Client](crate::Client) in the order they are generated!
#[derive_where(Debug; common::signal::ToClient<Server::Common>, Event<Server>, Ret)]
#[must_use]
pub struct Output<Server: self::Server, Ret> {
    /// Signals generated from the function call. These must be delivered to the indicated client.
    /// Signals to the same client must be delivered in order (in iteration order, and before any
    /// signals generated for that client in subsequent function calls). Signals between different
    /// clients are independent, and can be delivered in any order relative to each other.
    pub outbound: Vec<(player::Id, common::signal::ToClient<Server::Common>)>,
    /// Events which occurred during the function call. Any handling of these is optional.
    pub events: Vec<Event<Server>>,
    /// The main return value of the function.
    pub ret: Ret,
}

#[derive_where(Debug; common::signal::ToClient<Server::Common>, Event<Server>)]
struct Messaging<Server: self::Server> {
    pub outbound: Vec<(player::Id, common::signal::ToClient<Server::Common>)>,
    pub events: Vec<Event<Server>>,
}

impl<Server: self::Server> Messaging<Server> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<common::signal::ToClientInternal<Server::Common>>>(
        &mut self,
        player_id: player::Id,
        signal: S,
    ) {
        // TODO seq ids not yet implemented
        self.outbound.push((
            player_id,
            common::signal::ToClient {
                seq: 0,
                signal: signal.into(),
            },
        ));
    }

    pub fn push_event<E: Into<Event<Server>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Server, Ret> {
        let Self { outbound, events } = self;

        Output {
            outbound,
            events,
            ret,
        }
    }
}

/// The public interface for a server [Impl]
pub trait Server: crate::sealed::Sealed + Sized {
    /// The set of item types known by this server. See [trait@crate::State].
    type State: crate::State;

    /// The set of action types known by this server. See [trait@crate::Action].
    type Action: crate::Action<State = Self::State> + Clone;

    /// The context created during game initialization and provided to all game interactions.
    /// Usually, you will want this to be a type containing [IdT](crate::item::IdT)s of items
    /// created during game initialization, or an IdT of an item if you need it to be mutable.
    /// Mutating a Root item should be minimized, since it will interrupt any other actions other players
    /// are in the middle of.
    type Root: Clone;

    /// A type describing any initialization done for a new player. See [player::Init].
    type PlayerInit: crate::player::Init<State = Self::State, Action = Self::Action, Root = Self::Root>;

    /// A type describing all kinds of moves a player can make. See [Interaction](trait@crate::Interaction).
    type Interaction: crate::Interaction<
            State = Self::State,
            Action = Self::Action,
            Root = Self::Root,
            Trigger = <Self::Reaction as crate::Reaction>::Trigger,
        > + Clone;

    /// A type describing all possible server-side processing. See [Reaction](trait@crate::Reaction).
    type Reaction: crate::Reaction<
            State = Self::State,
            Action = Self::Action,
            Root = Self::Root,
            GameOutcome: Clone,
        >;

    /// The common types shared between a connected [Client](crate::Client) and [Server].
    type Common: crate::Common<
            State = Self::State,
            Action = Self::Action,
            Root = Self::Root,
            GameOutcome = <Self::Reaction as crate::Reaction>::GameOutcome,
            Interaction = Self::Interaction,
        >;

    /// Create a new server hosting a new game.
    /// `game_init` describes how to initialize the game. If game initialization fails,
    /// this method will return the error.
    fn init<GameInit>(
        game_init: GameInit,
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> Result<Self, error::InitError>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>;

    /// Create a new server from a loaded [Save].
    /// NOTE: this is not yet implemented!
    fn load(save: Save<Self>) -> Result<Self, error::LoadError>;

    /// Apply a signal from a [Client](crate::Client).
    /// Signals are how the client and server communicate. Signals between a client-server
    /// pair must be applied in the same order they are created.
    fn signal(
        &mut self,
        sender: player::Id,
        signal: common::signal::ToServer<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError>;

    /// Start a Reaction with a trigger as if it had been created by an [Interaction](trait@crate::Interaction).
    /// This allows manual modification of the game state without client intervention.
    /// Most server-driven updates should be triggered by interactions, but this could
    /// be used for implementing real-time timers, server-ops tools, or tests.
    fn manual_trigger(
        &mut self,
        trigger: <Self::Reaction as crate::Reaction>::Trigger,
    ) -> Result<Output<Self, ()>, error::ManualTriggerError>;

    /// Attempt to add a player to the game.
    /// The input to this function is determined by the server's [player::Init]. If player
    /// initialization succeeds, this function returns a [Seed](common::Seed) which is used to
    /// initialize a new [Client](crate::Client).
    fn add_player(
        &mut self,
        init_input: <Self::PlayerInit as crate::player::Init>::In,
    ) -> Result<Output<Self, common::Seed<Self::Common>>, error::AddPlayerError>;

    /// Save the current game state.
    /// [Server::load] can use this [Save] can create a new server instance at this
    /// point in the game.
    /// NOTE: [Server::load] is not yet implemented!
    fn save(&self) -> Result<Save<Self>, error::SaveError>
    where
        Self::PlayerInit: Clone,
        Self::Reaction: Clone;

    /// A unique identifier for this game. This is persisted through [Server::save] and [Server::load],
    /// so it is possible for multiple servers to have the same [game::Id].
    fn game_id(&self) -> game::Id;

    /// The game [Root](Server::Root)
    fn root(&self) -> &Self::Root;

    /// Read-only access to the authoritative game state [Storage](item::Storage)
    fn storage(&self) -> &Canonical<<Self::State as tagset::TagSet>::Repr, Self::State>;
}

/// A [Server] implementation for the given [Interaction](trait@crate::Interaction), [Reaction](trait@crate::Reaction), and [player::Init]
pub type Impl<Interaction, Reaction, PlayerInit> = ServerImpl<
    <<Interaction as crate::Interaction>::State as tagset::TagSet>::Repr,
    <Interaction as crate::Interaction>::State,
    <Interaction as crate::Interaction>::Action,
    <Interaction as crate::Interaction>::Root,
    Interaction,
    Reaction,
    PlayerInit,
>;

impl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit> crate::sealed::Sealed
    for ServerImpl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>
{
}

impl<Interaction, Reaction, PlayerInit> Server for Impl<Interaction, Reaction, PlayerInit>
where
    Interaction: crate::Interaction<Action: Clone, Root: Clone> + Clone,
    Reaction: crate::Reaction<
            State = Interaction::State,
            Action = Interaction::Action,
            Root = Interaction::Root,
            Trigger = Interaction::Trigger,
            GameOutcome: Clone,
        >,
    PlayerInit: crate::player::Init<
            State = Interaction::State,
            Action = Interaction::Action,
            Root = Interaction::Root,
        >,
{
    type State = Interaction::State;
    type Action = Interaction::Action;
    type Root = Interaction::Root;
    type PlayerInit = PlayerInit;
    type Interaction = Interaction;
    type Reaction = Reaction;

    type Common = crate::common::Impl<
        Self::State,
        Self::Action,
        Self::Root,
        Self::Interaction,
        <Self::Reaction as crate::Reaction>::GameOutcome,
    >;

    #[instrument(err, skip_all)]
    fn init<GameInit>(
        game_init: GameInit,
        player_init: Self::PlayerInit,
        reaction: Self::Reaction,
    ) -> Result<Self, error::InitError>
    where
        GameInit: game::Init<State = Self::State, Action = Self::Action, Root = Self::Root>,
    {
        let player_manager = player::Manager::new(player_init);

        let reservation = item::id::Reservation::all();

        let mut storage = Canonical::new();

        let context = game::init::Context {};

        let mut game_init_context = game::init::Error::prepare_context(&game_init);

        // If we fail at any point, there is no need to attempt undo since we scrapping the server anyway
        let mut interactor = Interactor::new(&mut storage, &reservation, context);
        let root = game_init
            .initialize(&mut interactor)
            .map_err(&mut game_init_context)?;

        let interactor_complete = interactor
            .complete(None::<game::init::Error>)
            .map_err(|e| e.initial_error)
            .map_err(&mut game_init_context)?;

        let log = log::Log::new();

        let mut inner = ImplInner {
            game_id: game::Id::new(),
            storage,
            player_manager,
            visibility: visibility::Manager::new(),
            log,
            reservation,
            reaction,
            error_state: common::error::FatalErrorState::default(),
            _interaction: PhantomData,
            _root: PhantomData,
        };

        let mut messaging = Messaging::new();

        inner
            .build_transaction(&root, &mut messaging, None, interactor_complete)
            .map_err(|re| game::init::Error::from(re.initial_error))
            .map_err(&mut game_init_context)?;

        let server = Self { root, inner };

        Ok(server)
    }

    #[instrument(err, skip_all)]
    fn load(save: Save<Self>) -> Result<Self, error::LoadError> {
        let Save {
            game_id,
            snapshot,
            next_transaction_id,
            reservation,
            player_manager,
            reaction,
        } = save;

        let reservation = reservation.reservation();

        let mut storage = Canonical::new();

        let root = snapshot.root().clone();
        snapshot.apply(&mut storage)?;

        let log = log::Log::new_with_next_id(next_transaction_id);

        let _server = Self {
            root,
            inner: ImplInner {
                game_id,
                storage,
                player_manager,
                visibility: visibility::Manager::new(),
                log,
                reservation,
                reaction,
                error_state: common::error::FatalErrorState::default(),
                _interaction: PhantomData,
                _root: PhantomData,
            },
        };

        todo!()
    }

    #[instrument(err, skip_all, fields(sender = sender.into_u32()))]
    fn signal(
        &mut self,
        sender: player::Id,
        arg: common::signal::ToServer<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError> {
        self.inner.error_state.check()?;

        let common::signal::ToServer { seq: _seq, signal } = arg;

        let mut messaging = Messaging::new();

        match signal {
            common::signal::ToServerInternal::ApplyInteraction(apply_interaction) => {
                self.inner
                    .apply_interaction(&self.root, &mut messaging, sender, apply_interaction)
                    .map_err(PseudoError::into_error)
                    .map_err(self.inner.error_state.make_fatal())?;
            }
        }

        Ok(messaging.into_output(()))
    }

    #[instrument(err, skip_all)]
    fn manual_trigger(
        &mut self,
        trigger: <Self::Reaction as crate::Reaction>::Trigger,
    ) -> Result<Output<Self, ()>, error::ManualTriggerError> {
        self.inner.error_state.check()?;

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

        self.inner
            .build_transaction(&self.root, &mut messaging, None, complete)
            .map_err(|re| {
                if re.is_recovered() {
                    error::ManualTriggerError::Reaction(re.initial_error)
                } else {
                    error::ManualTriggerError::Fatal(self.inner.error_state.make_fatal()(
                        re.map_with(PseudoError::into_error),
                    ))
                }
            })?;

        Ok(messaging.into_output(()))
    }

    #[instrument(err, skip_all, fields(player_id = tracing::field::Empty))]
    fn add_player(
        &mut self,
        init_input: <Self::PlayerInit as crate::player::Init>::In,
    ) -> Result<Output<Self, common::Seed<Self::Common>>, error::AddPlayerError> {
        self.inner.error_state.check()?;

        let mut messaging = Messaging::new();

        // TODO 640K ought to be enough for anybody
        let (reservation, reservation_range) = self
            .inner
            .reservation
            .split(1024 * 640)
            .expect("depleted id pool");

        let context = player::init::Context {
            root: &self.root,
            // To be overwritten in player::Manager::add
            player: player::Id::ZERO,
        };
        let interactor = Interactor::new(&mut self.inner.storage, &self.inner.reservation, context);

        let result = match self
            .inner
            .player_manager
            .add(interactor, reservation_range, init_input)
        {
            Ok(complete) => {
                let added_player_id = complete.context.player;
                tracing::Span::current().record("player_id", added_player_id.into_u32());

                self.inner
                    .build_transaction(&self.root, &mut messaging, None, complete)
                    .map(|transaction| (transaction, added_player_id))
                    .map_err(RecoverableError::map)
            }
            Err(err) => Err(err),
        };

        match result {
            // No reactions are currently permitted on init, so no GameOutcome is possible
            Ok((_transaction, added_player_id)) => {
                let game_id = self.inner.game_id;
                // TODO cache snapshots and reuse them if they are recent enough
                let snapshot = self.inner.create_snapshot(&self.root)?;
                let transactions = Transactions::new(self.inner.log.next_id());

                let client_init = common::Seed::<Self::Common> {
                    game_id,
                    local_player_id: added_player_id,
                    snapshot,
                    transactions,
                    reservation,
                };

                Ok(messaging.into_output(client_init))
            }
            Err(err) => {
                self.inner.player_manager.revert_add();
                // TODO the reservation is lost here
                if err.recovery_error.is_some() {
                    Err(error::AddPlayerError::Fatal(self
                        .inner
                        .error_state
                        .make_fatal()(
                        err.into_error()
                    )))
                } else {
                    Err(error::AddPlayerError::PlayerInit(err.initial_error))
                }
            }
        }
    }

    #[instrument(err, skip_all)]
    fn save(&self) -> Result<Save<Self>, error::SaveError>
    where
        PlayerInit: Clone,
        Reaction: Clone,
    {
        self.inner.error_state.check()?;

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

    fn game_id(&self) -> game::Id {
        self.inner.game_id
    }

    fn root(&self) -> &Self::Root {
        &self.root
    }

    fn storage(&self) -> &Canonical<<Self::State as tagset::TagSet>::Repr, Self::State> {
        &self.inner.storage
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ServerImpl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit> {
    inner: ImplInner<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>,
    root: Root,
}

// Needed for split borrows on root
#[derive_where(Debug; Repr, Action, Reaction, PlayerInit)]
struct ImplInner<Repr, State, Action, Root, Interaction, Reaction, PlayerInit> {
    game_id: game::Id,
    storage: item::storage::Canonical<Repr, State>,
    player_manager: player::Manager<PlayerInit>,
    log: Log<Action>,
    visibility: visibility::Manager,
    reservation: item::id::Reservation,
    reaction: Reaction,
    error_state: common::error::FatalErrorState,
    _interaction: PhantomData<Interaction>,
    _root: PhantomData<Root>,
}

impl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>
    ImplInner<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>
where
    State: crate::State<Repr = Repr>,
    Action: crate::Action<State = State> + Clone,
    Root: Clone,
    Interaction: crate::Interaction<
            State = State,
            Action = Action,
            Root = Root,
            Trigger = <Reaction as crate::Reaction>::Trigger,
        > + Clone,
    Reaction: crate::Reaction<State = State, Action = Action, Root = Root, GameOutcome: Clone>,
    PlayerInit: crate::player::Init<State = State, Action = Action, Root = Root>,
{
    #[instrument(err, skip_all)]
    fn apply_interaction(
        &mut self,
        root: &Root,
        messaging: &mut Messaging<
            ServerImpl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>,
        >,
        sender: player::Id,
        apply_interaction: common::signal::ApplyInteraction<
            <ServerImpl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit> as Server>::Common,
        >,
    ) -> action::Result<()> {
        let common::signal::ApplyInteraction {
            interaction:
                interaction::Staged {
                    interaction,
                    expected_versions,
                    pending_interaction_id,
                },
        } = apply_interaction;

        let result = (|| {
            // Each player is given a range of ids to use so that they don't step on each other's toes.
            // To protect against malicious clients, check each creation record belongs to that client.
            if let Err(err) = self
                .player_manager
                .get(sender)
                .check_created_ids(&expected_versions)
            {
                tracing::event!(name: "interaction_invalid_range", tracing::Level::INFO, { });

                return Err(RecoverableError::<interaction::Error>::new(err.into()));
            }

            let context = interaction::Context::new(root, sender);
            let mut interactor = Interactor::new(&mut self.storage, &self.reservation, context);
            let interaction_error = interaction
                .apply(&mut interactor)
                .map_err(|e| e.with_context(&interaction))
                .err();
            let complete = interactor.complete(interaction_error)?;
            // Auto-reject if versions don't match
            match expected_versions.diff(&complete.expected_versions) {
                Ok(()) => self
                    .build_transaction(root, messaging, Some(pending_interaction_id), complete)
                    .map_err(RecoverableError::map),
                Err(err) => {
                    tracing::event!(name: "interaction_version_conflict", tracing::Level::INFO, { });

                    Err(RecoverableError::new(
                        interaction::Error::new_validation_error(err),
                    ))
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
                messaging.push_signal(
                    sender,
                    common::signal::InteractionResult {
                        pending_interaction_id,
                        confirmed_transaction_id: None,
                    },
                );

                // First-level errors are expected here, so only report unrecoverable errors
                match err.recovery_error {
                    Some(err) => Err(*err),
                    None => Ok(()),
                }
            }
        }
    }

    #[instrument(err, skip_all)]
    fn build_transaction<Context, Output>(
        &mut self,
        root: &Root,
        messaging: &mut Messaging<
            ServerImpl<Repr, State, Action, Root, Interaction, Reaction, PlayerInit>,
        >,
        // This is a bit of a kludge to special case Interactions where a Client already has some of the transaction
        // applied locally and only needs log generated from Server Reactions
        pending_interaction: Option<interaction::Pending>,
        complete: interactor::Complete<Action, Context, Output>,
    ) -> Result<transaction::Confirmed<Action>, RecoverableError<action::Error>>
    where
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger>
            + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        // The player context appears in `add_player` and `Interaction`s. In either case,
        // this player does not need the full transaction. On `add_player`, the added player
        // will receive a snapshot and any log needed to bring it up to date, so we provide nothing
        // here. For an `Interaction`, the interacting player already has the log generated by
        // the interaction itself, but it needs anything generated by Server Reactions.
        // This case is tied to the `pending_interaction` parameter, and the unneeded log is determined
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
                        messaging.push_signal(
                            player_id,
                            common::signal::ConfirmedTransaction {
                                confirmed_transaction: confirmed_transaction.clone(),
                            },
                        );
                    }
                }

                if let Some(pending_interaction_id) = pending_interaction {
                    let Some(player_context) = player_context else {
                        unreachable!("Interactions must have a player context");
                    };

                    // Confirm the tentative transaction with the sender + the log generated by reactions
                    let confirmed_transaction_id = confirmed_transaction.id;
                    let mut records = confirmed_transaction.transaction.clone().into_records();
                    records.trim_start(initial_record_count);

                    messaging.push_signal(
                        player_context,
                        common::signal::InteractionResult {
                            pending_interaction_id,
                            confirmed_transaction_id: Some((confirmed_transaction_id, records)),
                        },
                    );
                }

                if let Some(game_outcome) = game_outcome {
                    for player_id in self.player_manager.iter() {
                        messaging.push_signal(
                            player_id,
                            common::signal::EndGame {
                                game_outcome: game_outcome.clone(),
                            },
                        )
                    }

                    messaging.push_event(event::GameComplete { game_outcome });
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
    ) -> Result<
        (
            transaction::Confirmed<Action>,
            Option<Reaction::GameOutcome>,
        ),
        RecoverableError<action::Error>,
    >
    where
        Context: interactor::PlayerContext,
        Output: interactor::TakeTriggers<Reaction::Trigger>
            + interactor::TakeGameOutcome<Reaction::Trigger>,
    {
        let Self {
            game_id: _id,
            storage,
            player_manager: _player_manager,
            log,
            visibility: _visibility_manager,
            reservation,
            reaction,
            error_state: _error_state,
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
            let mut interactor = Interactor::new(storage, reservation, reaction_context);
            if let Some(go) = game_outcome.take() {
                interactor.set_game_outcome(go);
            }

            let interactor_error = reaction.apply(&mut interactor, trigger).err();
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
                }
                Err(mut err) => {
                    // Only bother with undo if we haven't hit an unrecoverable error already
                    if err.is_recovered()
                        && let Err(undo_error) = all_undo_records.apply(storage)
                    {
                        // Undo failed, state is inconsistent
                        err.set_recovery_error(undo_error);
                    }

                    return Err(err);
                }
            }
        }

        let undo_transaction = Transaction::new(all_undo_records);
        let transaction_len = undo_transaction.records().len();

        let transaction_id = log.register_undo(undo_transaction);

        tracing::event!(name: "transaction", tracing::Level::DEBUG, id = transaction_id.get(), len = transaction_len);

        let do_transaction =
            transaction::Confirmed::new(transaction_id, Transaction::new(all_do_records));

        Ok((do_transaction, game_outcome))
    }

    fn create_snapshot(
        &self,
        root: &Root,
    ) -> Result<common::Snapshot<Repr, State, Root>, common::error::Save>
    where
        Root: Clone,
    {
        common::Snapshot::new(root.clone(), &self.storage)
    }
}
