pub mod apply_interaction;
mod core;
pub mod event;
use derive_where::derive_where;
pub use event::Event;
pub mod init;
pub mod revert_interaction;
pub mod signal;
pub mod stage_interaction;

use std::marker::PhantomData;
use crate::{interaction, interactor, item, log, player, server, Transaction};


#[derive_where(Debug; server::signal::Arg<Client::Common>, Event<Client>, Ret)]
pub struct Output<Client: self::Client, Ret> {
    pub outbound: Vec<server::signal::Arg<Client::Common>>,
    pub events: Vec<Event<Client>>,
    pub ret: Ret,
}

#[derive_where(Debug; server::signal::Arg<Client::Common>, Event<Client>)]
struct Messaging<Client: self::Client> {
    pub outbound: Vec<server::signal::Arg<Client::Common>>,
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
        self.outbound.push(server::signal::Arg { seq: 0, signal: signal.into() });
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

    fn init<Lookup>(
        lookup: &mut Lookup,
        init: init::Arg<Self::Common>,
    ) 
        -> init::Result<Self>
    where 
        Lookup: item::Lookup<State = Self::State>,    
    ;

    fn local_player_id(&self) -> player::Id;

    fn stage_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: stage_interaction::Arg<Self>) 
        -> stage_interaction::Result<Output<Self, stage_interaction::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn apply_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: apply_interaction::Arg)
        -> apply_interaction::Result<Output<Self, apply_interaction::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn revert_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: revert_interaction::Arg)
        -> revert_interaction::Result<Output<Self, revert_interaction::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;

    fn signal<Lookup>(&mut self, lookup: &mut Lookup, arg: signal::Arg<Self::Common>)
        -> signal::Result<Output<Self, signal::Ret>>
    where 
        Lookup: item::Lookup<State = Self::State>,
    ;
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
                snapshot,
                transactions,
                reservation,
                local_player_id,
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
                log,
                reservation,
                root,
                local_player_id,
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

    fn stage_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: stage_interaction::Arg<Self>) 
        -> stage_interaction::Result<Output<Self, stage_interaction::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let stage_interaction::Arg {
            interaction,
        } = arg;

        let state = self::Messaging::new();

        let context = interaction::Context::new(&self.core.root, self.core.local_player_id);
        let mut interactor = interaction::Interactor::new(lookup, &self.core.reservation, context);
        
        let interaction_error = interaction.apply(&mut interactor).err();
        let complete = interactor.complete(interaction_error)
            .map_err(crate::TempError::discard)?;
        
        let interactor::Complete {
            expected_versions,
            do_records: _do_records,
            undo_records,
            context: _context,
            output,
        } = complete;

        // The client can discard any Triggers, those are the server's responsibility
        // and the confirmed transaction message will contain any generated records
        let _ = output;
        
        let interaction = interaction::Staged {
            interaction,
            expected_versions,
        };
        let pending_transaction_id = self.core.log.stage_pending(interaction, Transaction::new(undo_records));

        Ok(state.into_output(stage_interaction::Ret {
            pending_transaction_id,
        }))
    }

    fn apply_interaction<Lookup>(&mut self, _lookup: &mut Lookup, arg: apply_interaction::Arg)
        -> apply_interaction::Result<Output<Self, apply_interaction::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let apply_interaction::Arg {
            pending_transaction_id,
        } = arg;

        let mut state = self::Messaging::new();

        let interactions = self.core.log.apply_pending(pending_transaction_id)?;

        for interaction in interactions {
            state.push_signal(server::signal::ApplyInteraction {
                interaction,
                pending_transaction_id,
            });
        }

        Ok(state.into_output(apply_interaction::Ret { }))
    }

    fn revert_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: revert_interaction::Arg)
        -> revert_interaction::Result<Output<Self, revert_interaction::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let revert_interaction::Arg {
            pending_transaction_id,
        } = arg;

        let state = self::Messaging::new();

        self.core.log.revert_pending(lookup, Some(pending_transaction_id))
            .map_err(crate::TempError::discard)?;

        Ok(state.into_output(revert_interaction::Ret { }))
    }

    fn signal<Lookup>(&mut self, lookup: &mut Lookup, arg: signal::Arg<Self::Common>)
        -> signal::Result<Output<Self, signal::Ret>>
    where
        Lookup: item::Lookup<State = Self::State>,
    {
        let signal::Arg {
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
}

impl<State, Action, Root, Interaction, GameOutcome> Impl<State, Action, Root, Interaction, GameOutcome> 
where 
    State: crate::State,
    Action: crate::Action<State = State>,
    Root: Clone,
    Interaction: crate::Interaction<State = State, Action = Action, Root = Root>,
{
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
            self.core.log.revert_pending(lookup, None)
                .map_err(crate::TempError::discard)?;
        }

        Ok(signal::Ret::new())
    }

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

    fn end_game<Lookup>(&mut self, state: &mut self::Messaging<Self>, lookup: &mut Lookup, end_game: signal::EndGame<<Self as Client>::Common>)
        -> signal::Result<signal::Ret>
    where
        Lookup: item::Lookup<State = State>,
    {
        let signal::EndGame {
            game_outcome,
        } = end_game;
        // The game is ending anyway, so if we become desynced, just ignore it
        let _ = self.core.log.revert_pending(lookup, None);

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
