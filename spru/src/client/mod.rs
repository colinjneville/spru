pub mod apply_interactions;
mod core;
pub mod event;
use derive_where::derive_where;
pub use event::Event;
pub mod init;
pub mod revert_interactions;
pub mod signal;
pub use signal::Signal;
use tracing::instrument;
pub mod stage_interaction;

use std::marker::PhantomData;
use crate::{game, interaction, interactor, item, log, player, server, transaction, Transaction};


#[derive_where(Debug; server::signal::Signal<Client::Common>, Event<Client>, Ret)]
pub struct Output<Client: self::Client, Ret> {
    pub outbound: Vec<server::signal::Signal<Client::Common>>,
    pub events: Vec<Event<Client>>,
    pub ret: Ret,
}

#[derive_where(Debug; server::signal::Signal<Client::Common>, Event<Client>)]
struct Messaging<Client: self::Client> {
    pub outbound: Vec<server::signal::Signal<Client::Common>>,
    pub events: Vec<Event<Client>>,
}

impl<Client: self::Client> Messaging<Client> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<server::signal::Internal<Client::Common>>>(&mut self, signal: S) {
        // TODO seq ids not yet implemented
        self.outbound.push(server::signal::Signal { seq: 0, signal: signal.into() });
    }

    pub fn push_event<E: Into<Event<Client>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Client, Ret> {
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

    /// Initialize a new [Client] 
    fn init<Lookup>(
        lookup: &mut Lookup,
        init: init::Arg<Self::Common>,
    ) 
        -> init::Result<Self>
    where 
        Lookup: item::Lookup<State = Self::State>,    
    ;

    fn local_player_id(&self) -> player::Id;

    fn stage_interaction<Lookup>(&mut self, lookup: &mut Lookup, interaction: Self::Interaction) 
        -> stage_interaction::Result<Output<Self, stage_interaction::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn apply_interactions<Lookup>(&mut self, lookup: &mut Lookup, pending_transaction_id: Option<transaction::Pending>)
        -> apply_interactions::Result<Output<Self, apply_interactions::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn revert_interactions<Lookup>(&mut self, lookup: &mut Lookup, pending_transaction_id: Option<transaction::Pending>)
        -> revert_interactions::Result<Output<Self, revert_interactions::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn signal<Lookup>(&mut self, lookup: &mut Lookup, arg: signal::Signal<Self::Common>)
        -> signal::Result<Output<Self, signal::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn game_id(&self) -> game::Id;
}

impl<State, Action, Root, Interaction, GameOutcome> Client for Impl<State, Action, Root, Interaction, GameOutcome>
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
        Self::Interaction
    >;

    #[instrument(err, skip_all, fields(local_player_id = init.local_player_id.into_u32()))]
    fn init<Lookup>(
        lookup: &mut Lookup,
        init: init::Arg<Self::Common>,
    ) 
        -> init::Result<Self>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let core = {
            let init::Arg {
                game_id,
                local_player_id,
                snapshot,
                transactions,
                reservation,
            } = init;

            let root = snapshot.root()
                .clone();

            snapshot.apply(lookup)
                .map_err(crate::TempError::discard)?;

            let mut log = log::Client::new(transactions.next_id());
            
            for transaction in transactions.into_iter() {
                log.apply_confirmed(lookup, transaction)
                    .map_err(crate::TempError::discard)?;
            }

            core::Core {
                game_id,
                local_player_id,
                log,
                reservation,
                root,
            }
        };


        Ok(Self {
            core,
            _game_outcome: PhantomData,
            _state: PhantomData,
        })
    }

    fn local_player_id(&self) -> player::Id {
        self.core.local_player_id
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn stage_interaction<Lookup>(&mut self, lookup: &mut Lookup, interaction: Self::Interaction) 
        -> stage_interaction::Result<Output<Self, stage_interaction::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let state = self::Messaging::new();

        let context = interaction::Context::new(&self.core.root, self.core.local_player_id);
        let mut interactor = interaction::Interactor::new(lookup, &self.core.reservation, context);
        
        let interaction_error = interaction.apply(&mut interactor)
            .map_err(|e| e.with_context(&interaction))
            .err();
        let complete = interactor.complete(interaction_error)?;
        
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
        
        let pending_transaction_id = self.core.log.stage_pending(interaction, expected_versions, Transaction::new(undo_records));
        
        tracing::info!(name: "stage_interaction_success", pending_transaction_id = pending_transaction_id.0.get(), "apply_interactions succeeded");
        Ok(state.into_output(stage_interaction::Ret {
            pending_transaction_id,
        }))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), pending_transaction_id = pending_transaction_id.map(|p| p.0.get())))]
    fn apply_interactions<Lookup>(&mut self, _lookup: &mut Lookup, pending_transaction_id: Option<transaction::Pending>)
        -> apply_interactions::Result<Output<Self, apply_interactions::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let mut state = self::Messaging::new();

        let interactions = self.core.log.apply_pending(pending_transaction_id)?;
        if !interactions.is_empty() {
            for interaction in interactions {
                state.push_signal(server::signal::ApplyInteraction {
                    interaction,
                });
            }
        }

        tracing::info!(name: "apply_interactions_success", "apply_interactions succeeded");
        Ok(state.into_output(apply_interactions::Ret { }))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), pending_transaction_id = pending_transaction_id.map(|p| p.0.get())))]
    fn revert_interactions<Lookup>(&mut self, lookup: &mut Lookup, pending_transaction_id: Option<transaction::Pending>)
        -> revert_interactions::Result<Output<Self, revert_interactions::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let state = self::Messaging::new();

        self.core.log.revert_pending(lookup, pending_transaction_id, false)
            .map_err(crate::TempError::discard)?;

        tracing::info!(name: "revert_interactions_success", "revert_interactions succeeded");
        Ok(state.into_output(revert_interactions::Ret { }))
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn signal<Lookup>(&mut self, lookup: &mut Lookup, arg: signal::Signal<Self::Common>)
        -> signal::Result<Output<Self, signal::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let signal::Signal {
            seq: _seq,
            signal,
        } = arg;

        let mut state = self::Messaging::new();

        let ret = match signal {
            signal::Internal::InteractionResult(interaction_result) => self.interaction_result(&mut state, lookup, interaction_result)?,
            signal::Internal::ConfirmedTransaction(confirmed_transaction) => self.confirmed_transaction(&mut state, lookup, confirmed_transaction)?,
            signal::Internal::EndGame(end_game) => self.end_game(&mut state, lookup, end_game)?,
        };

        Ok(state.into_output(ret))
    }

    fn game_id(&self) -> game::Id {
        self.core.game_id
    }
}

impl<State, Action, Root, Interaction, GameOutcome> Impl<State, Action, Root, Interaction, GameOutcome> 
where 
    State: crate::State,
    Action: crate::Action<State = State>,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root>,
{
    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn interaction_result<Lookup>(&mut self, _state: &mut self::Messaging<Self>, lookup: &mut Lookup, interaction_result: signal::InteractionResult<<Self as Client>::Common>)
        -> signal::Result<signal::Ret>
    where
        Lookup: item::Lookup<State = State>,
    {
        let signal::InteractionResult {
            pending_transaction_id,
            confirmed_transaction_id,
        } = interaction_result;

        if let Some((confirmed_transaction_id, extra_records)) = confirmed_transaction_id {
            self.core.log.confirm_pending(lookup, pending_transaction_id, confirmed_transaction_id, &extra_records)
                .map_err(crate::TempError::discard)?;
        } else {
            self.core.log.revert_pending(lookup, None, true)
                .map_err(crate::TempError::discard)?;
        }

        Ok(signal::Ret::new())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id(), transaction_id = confirmed_transaction.confirmed_transaction.id.get()))]
    fn confirmed_transaction<Lookup>(&mut self, _state: &mut self::Messaging<Self>, lookup: &mut Lookup, confirmed_transaction: signal::ConfirmedTransaction<<Self as Client>::Common>)
        -> signal::Result<signal::Ret>
    where
        Lookup: item::Lookup<State = State>,
    {
        let signal::ConfirmedTransaction {
            confirmed_transaction,
        } = confirmed_transaction;

        self.core.log.apply_confirmed(lookup, confirmed_transaction)
            .map_err(crate::TempError::discard)?;

        Ok(signal::Ret::new())
    }

    #[instrument(err, skip_all, fields(local_player_id = self.trace_id()))]
    fn end_game<Lookup>(&mut self, state: &mut self::Messaging<Self>, lookup: &mut Lookup, end_game: signal::EndGame<<Self as Client>::Common>)
        -> signal::Result<signal::Ret>
    where
        Lookup: item::Lookup<State = State>,
    {
        let signal::EndGame {
            game_outcome,
        } = end_game;
        // The game is ending anyway, so if we become desynced, just ignore it
        let _ = self.core.log.revert_pending(lookup, None, true);

        state.events.push(event::GameComplete { game_outcome }.into());

        Ok(signal::Ret::new())
    }
}


#[derive(Debug)]
pub struct Impl<State, Action, Root, Interaction, GameOutcome> {
    core: core::Core<Action, Root, Interaction>,
    _game_outcome: PhantomData<fn() -> GameOutcome>,
    _state: PhantomData<fn() -> State>,
}

impl<State, Action, Root, Interaction, GameOutcome> Impl<State, Action, Root, Interaction, GameOutcome> {
    fn trace_id(&self) -> u32 {
        self.core.local_player_id.into_u32()
    }
}

