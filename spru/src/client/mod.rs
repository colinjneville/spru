pub mod error;
pub mod event;
use derive_where::derive_where;
pub use event::Event;
mod log;
use log::Log;

use tracing::instrument;

use crate::{
    Transaction,
    common::{
        self,
        error::{FatalError, PseudoError},
    },
    game, interaction, interactor, item, player,
};
use std::marker::PhantomData;

/// A [Client] implementation for the given [Interaction](trait@crate::Interaction) and `GameOutcome`
pub type Impl<Interaction, GameOutcome> = ClientImpl<
    <Interaction as crate::Interaction>::State,
    <Interaction as crate::Interaction>::Action,
    <Interaction as crate::Interaction>::Root,
    Interaction,
    GameOutcome,
>;

/// Outputs of [Client] functions.
/// Notably, you are responsible for delivering all signals in [Output::outbound] to
/// the [Server](crate::Server) in the order they are generated!
#[derive_where(Debug; common::signal::ToServer<Client::Common>, Event<Client>, Ret)]
pub struct Output<Client: self::Client, Ret> {
    /// Signals generated from the function call. These must be delivered to the server.
    /// Signals from this client must be delivered in order (in iteration order, and before
    /// any signals generated in subsequent function calls).
    pub outbound: Vec<common::signal::ToServer<Client::Common>>,
    /// Events which occurred during the function call. Any handling of these is optional.
    pub events: Vec<Event<Client>>,
    /// The main return value of the function.
    pub ret: Ret,
}

#[derive_where(Debug; common::signal::ToServer<Client::Common>, Event<Client>)]
struct Messaging<Client: self::Client> {
    pub outbound: Vec<common::signal::ToServer<Client::Common>>,
    pub events: Vec<Event<Client>>,
}

impl<Client: self::Client> Messaging<Client> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<common::signal::ToServerInternal<Client::Common>>>(
        &mut self,
        signal: S,
    ) {
        // TODO seq ids not yet implemented
        self.outbound.push(common::signal::ToServer {
            seq: 0,
            signal: signal.into(),
        });
    }

    pub fn push_event<E: Into<Event<Client>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Client, Ret> {
        let Self { outbound, events } = self;

        Output {
            outbound,
            events,
            ret,
        }
    }
}

/// The public interface for a client [Impl]  
/// Most operations require access to the storage for this client, see [item::Storage].
pub trait Client: crate::sealed::Sealed + Sized {
    /// The set of item types known by this client. See [trait@crate::State].
    type State: crate::State;

    /// The set of action types known by this client. See [trait@crate::Action].
    type Action: crate::Action<State = Self::State>;

    /// The context created during game initialization and provided to all game interactions.  
    /// Usually, you will want this to be a type containing [item::IdT]s of items
    /// created during game initialization, or an IdT of an item if you need it to be mutable.  
    /// Mutating a Root item should be minimized, since it will interrupt any other actions other players
    /// are in the middle of.
    type Root: Clone;

    /// A type describing all kinds of moves a player can make. See [Interaction](trait@crate::Interaction).
    type Interaction: crate::Interaction<State = Self::State, Action = Self::Action, Root = Self::Root>;

    /// The final result of a game, usually containing the winner
    type GameOutcome;

    /// The common types shared between a connected [Client] and [Server](crate::Server).
    type Common: crate::Common<
            State = Self::State,
            Action = Self::Action,
            Root = Self::Root,
            GameOutcome = Self::GameOutcome,
            Interaction = Self::Interaction,
        >;

    /// Initialize a client from a [Seed](common::Seed) from a server.
    fn init<Storage>(
        storage: &mut Storage,
        seed: common::Seed<Self::Common>,
    ) -> Result<Self, error::InitError>
    where
        Storage: item::Storage<State = Self::State>;

    /// The player this client is for
    fn local_player_id(&self) -> player::Id;

    /// Run an [Interaction](trait@crate::Interaction) locally.  
    /// The game state will be updated locally with the interaction. Changes applied
    /// locally must eventually be submitted to the server with [Client::apply_interactions],
    /// or cancelled with [Client::revert_interactions].  
    /// If the server confirms a transaction which conflicts with out local interactions,
    /// they will be rolled back automatically.  
    /// If the interaction is invalid, the client will attempt to rollback the
    /// changes. If rollback is unsuccessful due to an implementation error,
    /// a [FatalError] will be returned. Otherwise, a non-fatal error is returned,
    /// and the game state will be equivalent to before calling this function.
    fn stage_interaction<Storage>(
        &mut self,
        storage: &mut Storage,
        interaction: Self::Interaction,
    ) -> Result<Output<Self, interaction::Pending>, error::StageInteractionError>
    where
        Storage: item::Storage<State = Self::State>;

    /// Send interactions staged locally to the server for confirmation.  
    /// If `pending_interaction_id` is `None`, all staged interactions will be sent.
    /// If it is `Some`, that interaction and all staged interactions preceding it
    /// will be sent.  
    ///
    /// This function only sends the interactions to the server, an `Ok` result does
    /// not mean the interactions were accepted.  
    ///
    /// Returns the number of applied interactions
    fn apply_interactions<Storage>(
        &mut self,
        storage: &mut Storage,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, usize>, error::ApplyInteractionError>
    where
        Storage: item::Storage<State = Self::State>;

    /// Revert locally staged interactions.  
    /// If `pending_interaction_id` is `None`, all staged interactions will be reverted.
    /// If it is `Some`, that interaction and all staged interactions *after* it will be reverted.
    ///
    /// Returns the number of reverted interactions
    fn revert_interactions<Storage>(
        &mut self,
        storage: &mut Storage,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, usize>, error::RevertInteractionError>
    where
        Storage: item::Storage<State = Self::State>;

    /// Apply a signal from a [Server](crate::Server).  
    /// Signals are how the client and server communicate. Signals between a client-server
    /// pair must be applied in the same order they are created.
    fn signal<Storage>(
        &mut self,
        storage: &mut Storage,
        arg: common::signal::ToClient<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError>
    where
        Storage: item::Storage<State = Self::State>;

    /// A unique identifier for this game.
    fn game_id(&self) -> game::Id;

    /// The game [Root](Client::Root)
    fn root(&self) -> &Self::Root;

    /// [Interaction](trait@crate::Interaction)s which have been staged, but not yet applied or reverted
    fn pending_interactions(&self) -> impl Iterator<Item = interaction::Pending>;
}

impl<State, Action, Root, Interaction, GameOutcome> crate::sealed::Sealed
    for ClientImpl<State, Action, Root, Interaction, GameOutcome>
{
}

impl<Interaction, GameOutcome> Client for Impl<Interaction, GameOutcome>
where
    Interaction: crate::Interaction<
            State: crate::State,
            Action: crate::Action<State = Interaction::State>,
            Root: Clone,
        >,
{
    type State = Interaction::State;
    type Action = Interaction::Action;
    type Root = Interaction::Root;
    type Interaction = Interaction;
    type GameOutcome = GameOutcome;

    type Common = crate::common::CommonImpl<Self::Interaction, Self::GameOutcome>;

    #[instrument(err, skip_all, fields(local_player_id = seed.local_player_id.into_u32()))]
    fn init<Storage>(
        storage: &mut Storage,
        seed: common::Seed<Self::Common>,
    ) -> Result<Self, error::InitError>
    where
        Storage: item::Storage<State = Self::State>,
    {
        let inner = {
            let common::Seed {
                game_id,
                local_player_id,
                snapshot,
                transactions,
                reservation,
            } = seed;

            let mut error_state = common::error::FatalErrorState::default();

            let root = snapshot.root().clone();

            snapshot.apply(storage).map_err(error_state.make_fatal())?;

            let mut log = log::Log::new(transactions.next_id());

            for transaction in transactions.into_iter() {
                log.apply_confirmed(storage, transaction)
                    .map_err(error_state.make_fatal())?;
            }

            ImplInner {
                game_id,
                local_player_id,
                log,
                reservation,
                root,
                error_state,
            }
        };

        Ok(Self {
            inner,
            _game_outcome: PhantomData,
            _state: PhantomData,
        })
    }

    fn local_player_id(&self) -> player::Id {
        self.inner.local_player_id
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn stage_interaction<Storage>(
        &mut self,
        storage: &mut Storage,
        interaction: Self::Interaction,
    ) -> Result<Output<Self, interaction::Pending>, error::StageInteractionError>
    where
        Storage: item::Storage<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let state = self::Messaging::new();

        let context = interaction::Context::new(&self.inner.root, self.inner.local_player_id);
        let mut interactor = interaction::Interactor::<Storage, Self::Interaction>::new(
            storage,
            &self.inner.reservation,
            context,
        );

        let interaction_error = interaction
            .apply(&mut interactor)
            .map_err(|e| e.with_context(&interaction))
            .err();
        let complete = interactor.complete(interaction_error).map_err(|re| {
            if re.is_recovered() {
                error::StageInteractionError::Interaction(re.initial_error)
            } else {
                error::StageInteractionError::Fatal(self.inner.error_state.make_fatal()(
                    re.map_with(PseudoError::into_error),
                ))
            }
        })?;

        let interactor::Complete {
            expected_versions,
            do_records: _do_records,
            undo_records,
            context: _context,
            output,
        } = complete;

        // The client can discard any Triggers, those are the server's responsibility
        // and the confirmed transaction signal will contain any generated records
        let _ = output;

        let pending_interaction_id = self.inner.log.stage_pending(
            interaction,
            expected_versions,
            Transaction::new(undo_records),
        );

        tracing::info!(name: "stage_interaction_success", pending_interaction_id = pending_interaction_id.into_u32(), "apply_interactions succeeded");
        Ok(state.into_output(pending_interaction_id))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), pending_interaction_id = pending_interaction_id.map(interaction::Pending::into_u32)))]
    fn apply_interactions<Storage>(
        &mut self,
        _s: &mut Storage,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, usize>, error::ApplyInteractionError>
    where
        Storage: item::Storage<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let mut state = self::Messaging::new();

        let mut count = 0;

        let interactions = self.inner.log.apply_pending(pending_interaction_id)?;
        if !interactions.is_empty() {
            for interaction in interactions {
                state.push_signal(common::signal::ApplyInteraction { interaction });
                count += 1;
            }
        }

        tracing::info!(name: "apply_interactions_success", "apply_interactions succeeded");
        Ok(state.into_output(count))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), pending_interaction_id = pending_interaction_id.map(interaction::Pending::into_u32)))]
    fn revert_interactions<Storage>(
        &mut self,
        storage: &mut Storage,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, usize>, error::RevertInteractionError>
    where
        Storage: item::Storage<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let state = self::Messaging::new();

        let count = self
            .inner
            .log
            .revert_pending(storage, pending_interaction_id, false)
            .map_err(PseudoError::into_error)
            .map_err(self.inner.error_state.make_fatal())?;

        tracing::info!(name: "revert_interactions_success", "revert_interactions succeeded");
        Ok(state.into_output(count))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn signal<Storage>(
        &mut self,
        storage: &mut Storage,
        arg: common::signal::ToClient<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError>
    where
        Storage: item::Storage<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let common::signal::ToClient { seq: _seq, signal } = arg;

        let mut state = self::Messaging::new();

        let () = match signal {
            common::signal::ToClientInternal::InteractionResult(interaction_result) => {
                self.interaction_result(&mut state, storage, interaction_result)?
            }
            common::signal::ToClientInternal::ConfirmedTransaction(confirmed_transaction) => {
                self.confirmed_transaction(&mut state, storage, confirmed_transaction)?
            }
            common::signal::ToClientInternal::EndGame(end_game) => {
                self.end_game(&mut state, storage, end_game)?
            }
        };

        Ok(state.into_output(()))
    }

    fn game_id(&self) -> game::Id {
        self.inner.game_id
    }

    fn root(&self) -> &Self::Root {
        &self.inner.root
    }

    fn pending_interactions(&self) -> impl Iterator<Item = interaction::Pending> {
        self.inner.log.pending_interactions()
    }
}

impl<State, Action, Root, Interaction, GameOutcome>
    ClientImpl<State, Action, Root, Interaction, GameOutcome>
where
    State: crate::State,
    Action: crate::Action<State = State>,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root>,
{
    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn interaction_result<Storage>(
        &mut self,
        messaging: &mut self::Messaging<Self>,
        storage: &mut Storage,
        interaction_result: common::signal::InteractionResult<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Storage: item::Storage<State = State>,
    {
        let common::signal::InteractionResult {
            pending_interaction_id,
            confirmed_transaction_id,
        } = interaction_result;

        messaging.push_event(event::InteractionResult::<Self> {
            pending_interaction_id,
            confirmed: confirmed_transaction_id.is_some(),
            _client: PhantomData,
        });

        if let Some((confirmed_transaction_id, extra_records)) = confirmed_transaction_id {
            self.inner
                .log
                .confirm_pending(
                    storage,
                    pending_interaction_id,
                    confirmed_transaction_id,
                    &extra_records,
                )
                .map_err(self.inner.error_state.make_fatal())?;
        } else {
            self.inner
                .log
                .revert_pending(storage, None, true)
                .map_err(PseudoError::into_error)
                .map_err(self.inner.error_state.make_fatal())?;
        }

        Ok(())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), transaction_id = confirmed_transaction.confirmed_transaction.id.get()))]
    fn confirmed_transaction<Storage>(
        &mut self,
        _messaging: &mut self::Messaging<Self>,
        storage: &mut Storage,
        confirmed_transaction: common::signal::ConfirmedTransaction<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Storage: item::Storage<State = State>,
    {
        let common::signal::ConfirmedTransaction {
            confirmed_transaction,
        } = confirmed_transaction;

        self.inner
            .log
            .apply_confirmed(storage, confirmed_transaction)
            .map_err(self.inner.error_state.make_fatal())?;

        Ok(())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn end_game<Storage>(
        &mut self,
        messaging: &mut self::Messaging<Self>,
        storage: &mut Storage,
        end_game: common::signal::EndGame<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Storage: item::Storage<State = State>,
    {
        let common::signal::EndGame { game_outcome } = end_game;
        // The game is ending anyway, so if we become desynced, just ignore it
        let _ = self.inner.log.revert_pending(storage, None, true);

        messaging
            .events
            .push(event::GameComplete { game_outcome }.into());

        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ClientImpl<State, Action, Root, Interaction, GameOutcome> {
    inner: ImplInner<Action, Root, Interaction>,
    _game_outcome: PhantomData<fn() -> GameOutcome>,
    _state: PhantomData<fn() -> State>,
}

impl<State, Action, Root, Interaction, GameOutcome>
    ClientImpl<State, Action, Root, Interaction, GameOutcome>
{
    fn trace_id(&self) -> u32 {
        self.inner.local_player_id.into_u32()
    }
}

#[derive(Debug)]
struct ImplInner<Action, Root, Interaction> {
    pub(crate) game_id: game::Id,
    pub(crate) local_player_id: player::Id,
    pub(crate) log: Log<Action, Interaction>,
    pub(crate) root: Root,
    pub(crate) reservation: item::id::Reservation,
    pub(crate) error_state: common::error::FatalErrorState,
}
