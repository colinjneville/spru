pub mod error;
pub mod event;
use derive_where::derive_where;
pub use event::Event;
mod log;
use log::Log;
pub mod signal;
use tracing::instrument;

use crate::{
    Transaction,
    common::{
        self,
        error::{FatalError, PsuedoError},
    },
    game, interaction, interactor, item, player,
};
use std::marker::PhantomData;

#[derive_where(Debug; common::signal::ToServer<Client::Common>, Event<Client>, Ret)]
pub struct Output<Client: self::Client, Ret> {
    pub outbound: Vec<common::signal::ToServer<Client::Common>>,
    pub events: Vec<Event<Client>>,
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

pub trait Client: Sized {
    type State: crate::State;
    type Action: crate::Action<State = Self::State>;
    type Root: Clone;
    type GameOutcome;
    type Interaction: crate::Interaction<State = Self::State, Action = Self::Action, Root = Self::Root>;

    type Common: crate::Common<
            State = Self::State,
            Action = Self::Action,
            Root = Self::Root,
            GameOutcome = Self::GameOutcome,
            Interaction = Self::Interaction,
        >;

    fn init<Lookup>(
        lookup: &mut Lookup,
        seed: common::Seed<Self::Common>,
    ) -> Result<Self, error::InitError>
    where
        Lookup: item::Lookup<State = Self::State>;

    fn local_player_id(&self) -> player::Id;

    fn stage_interaction<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        interaction: Self::Interaction,
    ) -> Result<Output<Self, interaction::Pending>, error::StageInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>;

    fn apply_interactions<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, ()>, error::ApplyInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>;

    fn revert_interactions<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, ()>, error::RevertInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>;

    fn signal<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        arg: common::signal::ToClient<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError>
    where
        Lookup: item::Lookup<State = Self::State>;

    fn game_id(&self) -> game::Id;
}

impl<State, Action, Root, Interaction, GameOutcome> Client
    for Impl<State, Action, Root, Interaction, GameOutcome>
where
    State: crate::State,
    Action: crate::Action<State = State>,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root>,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type Interaction = Interaction;
    type GameOutcome = GameOutcome;

    type Common = crate::common::Impl<
        Self::State,
        Self::Action,
        Self::Root,
        Self::GameOutcome,
        Self::Interaction,
    >;

    #[instrument(err, skip_all, fields(local_player_id = seed.local_player_id.into_u32()))]
    fn init<Lookup>(
        lookup: &mut Lookup,
        seed: common::Seed<Self::Common>,
    ) -> Result<Self, error::InitError>
    where
        Lookup: item::Lookup<State = Self::State>,
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

            snapshot.apply(lookup).map_err(error_state.make_fatal())?;

            let mut log = log::Log::new(transactions.next_id());

            for transaction in transactions.into_iter() {
                log.apply_confirmed(lookup, transaction)
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
    fn stage_interaction<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        interaction: Self::Interaction,
    ) -> Result<Output<Self, interaction::Pending>, error::StageInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let state = self::Messaging::new();

        let context = interaction::Context::new(&self.inner.root, self.inner.local_player_id);
        let mut interactor = interaction::Interactor::new(lookup, &self.inner.reservation, context);

        let interaction_error = interaction
            .apply(&mut interactor)
            .map_err(|e| e.with_context(&interaction))
            .err();
        let complete = interactor.complete(interaction_error).map_err(|re| {
            if re.is_recovered() {
                error::StageInteractionError::Interaction(re.initial_error)
            } else {
                error::StageInteractionError::Fatal(self.inner.error_state.make_fatal()(
                    re.map_with(PsuedoError::into_error),
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
    fn apply_interactions<Lookup>(
        &mut self,
        _lookup: &mut Lookup,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, ()>, error::ApplyInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let mut state = self::Messaging::new();

        let interactions = self.inner.log.apply_pending(pending_interaction_id)?;
        if !interactions.is_empty() {
            for interaction in interactions {
                state.push_signal(common::signal::ApplyInteraction { interaction });
            }
        }

        tracing::info!(name: "apply_interactions_success", "apply_interactions succeeded");
        Ok(state.into_output(()))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), pending_interaction_id = pending_interaction_id.map(interaction::Pending::into_u32)))]
    fn revert_interactions<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Output<Self, ()>, error::RevertInteractionError>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let state = self::Messaging::new();

        self.inner
            .log
            .revert_pending(lookup, pending_interaction_id, false)
            .map_err(PsuedoError::into_error)
            .map_err(self.inner.error_state.make_fatal())?;

        tracing::info!(name: "revert_interactions_success", "revert_interactions succeeded");
        Ok(state.into_output(()))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn signal<Lookup>(
        &mut self,
        lookup: &mut Lookup,
        arg: common::signal::ToClient<Self::Common>,
    ) -> Result<Output<Self, ()>, error::SignalError>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        self.inner.error_state.check()?;

        let common::signal::ToClient { seq: _seq, signal } = arg;

        let mut state = self::Messaging::new();

        let () = match signal {
            common::signal::ToClientInternal::InteractionResult(interaction_result) => {
                self.interaction_result(&mut state, lookup, interaction_result)?
            }
            common::signal::ToClientInternal::ConfirmedTransaction(confirmed_transaction) => {
                self.confirmed_transaction(&mut state, lookup, confirmed_transaction)?
            }
            common::signal::ToClientInternal::EndGame(end_game) => {
                self.end_game(&mut state, lookup, end_game)?
            }
        };

        Ok(state.into_output(()))
    }

    fn game_id(&self) -> game::Id {
        self.inner.game_id
    }
}

impl<State, Action, Root, Interaction, GameOutcome>
    Impl<State, Action, Root, Interaction, GameOutcome>
where
    State: crate::State,
    Action: crate::Action<State = State>,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root>,
{
    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn interaction_result<Lookup>(
        &mut self,
        messaging: &mut self::Messaging<Self>,
        lookup: &mut Lookup,
        interaction_result: common::signal::InteractionResult<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Lookup: item::Lookup<State = State>,
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
                    lookup,
                    pending_interaction_id,
                    confirmed_transaction_id,
                    &extra_records,
                )
                .map_err(self.inner.error_state.make_fatal())?;
        } else {
            self.inner
                .log
                .revert_pending(lookup, None, true)
                .map_err(PsuedoError::into_error)
                .map_err(self.inner.error_state.make_fatal())?;
        }

        Ok(())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), transaction_id = confirmed_transaction.confirmed_transaction.id.get()))]
    fn confirmed_transaction<Lookup>(
        &mut self,
        _messaging: &mut self::Messaging<Self>,
        lookup: &mut Lookup,
        confirmed_transaction: common::signal::ConfirmedTransaction<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Lookup: item::Lookup<State = State>,
    {
        let common::signal::ConfirmedTransaction {
            confirmed_transaction,
        } = confirmed_transaction;

        self.inner
            .log
            .apply_confirmed(lookup, confirmed_transaction)
            .map_err(self.inner.error_state.make_fatal())?;

        Ok(())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn end_game<Lookup>(
        &mut self,
        messaging: &mut self::Messaging<Self>,
        lookup: &mut Lookup,
        end_game: common::signal::EndGame<<Self as Client>::Common>,
    ) -> Result<(), FatalError>
    where
        Lookup: item::Lookup<State = State>,
    {
        let common::signal::EndGame { game_outcome } = end_game;
        // The game is ending anyway, so if we become desynced, just ignore it
        let _ = self.inner.log.revert_pending(lookup, None, true);

        messaging
            .events
            .push(event::GameComplete { game_outcome }.into());

        Ok(())
    }
}

#[derive(Debug)]
pub struct Impl<State, Action, Root, Interaction, GameOutcome> {
    inner: ImplInner<Action, Root, Interaction>,
    _game_outcome: PhantomData<fn() -> GameOutcome>,
    _state: PhantomData<fn() -> State>,
}

impl<State, Action, Root, Interaction, GameOutcome>
    Impl<State, Action, Root, Interaction, GameOutcome>
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
