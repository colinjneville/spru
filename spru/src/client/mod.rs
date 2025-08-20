pub mod apply_interaction;
pub mod event;
use std::marker::PhantomData;

pub use event::Event;
pub mod init;
pub mod revert_interaction;
pub mod signal;
pub mod stage_interaction;

use crate::{interaction, item, log, player, server, state, Interactor, Transaction};


#[derive(Debug)]
pub struct Output<Interaction, GameOutcome, Ret> {
    pub outbound: Vec<server::signal::Arg<Interaction>>,
    pub events: Vec<Event<GameOutcome>>,
    pub ret: Ret,
}

#[derive(Debug)]
struct State<Interaction, GameOutcome> {
    pub outbound: Vec<server::signal::Arg<Interaction>>,
    pub events: Vec<Event<GameOutcome>>,
}

impl<Interaction, GameOutcome> State<Interaction, GameOutcome> {
    pub fn new() -> Self {
        Self {
            outbound: vec![],
            events: vec![],
        }
    }

    pub fn push_signal<S: Into<server::signal::Internal<Interaction>>>(&mut self, signal: S) {
        self.outbound.push(server::signal::Arg { signal: signal.into() });
    }

    pub fn push_event<E: Into<Event<GameOutcome>>>(&mut self, event: E) {
        self.events.push(event.into());
    }

    pub fn into_output<Ret>(self, ret: Ret) -> Output<Interaction, GameOutcome, Ret> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Id(usize);

#[derive(Debug)]
pub struct Client<Action, Root, Interaction, GameOutcome> {
    log: log::Client<Action, Interaction>,
    root: Root,
    reservation: item::id::Reservation,
    local_player_id: player::Id,
    _p: PhantomData<fn(GameOutcome) -> GameOutcome>,
}

impl<Action, Root, Interaction, GameOutcome> Client<Action, Root, Interaction, GameOutcome> {
    pub fn new_request<PlayerInitIn>(player_init_input: PlayerInitIn) -> server::add_player::Arg<PlayerInitIn> {
        server::add_player::Arg {
            init_input: player_init_input,
        }
    }

    pub fn init<State, Lookup>(
        lookup: &mut Lookup,
        init: init::Arg<State, Action, Root>,
    ) -> Result<Self, init::Error>
    where 
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
        State: crate::State<Lookup, Repr: TryFrom<state::Index>>,
        Root: Clone,
    {
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

        Ok(Self {
            log,
            reservation,
            root,
            local_player_id,
            _p: PhantomData::default(),
        })
    }

    pub fn local_player_id(&self) -> player::Id {
        self.local_player_id
    }

    pub fn stage_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: stage_interaction::Arg<Interaction>) 
        -> Result<Output<Interaction, GameOutcome, stage_interaction::Ret>, stage_interaction::Error>
    where
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
        Interaction: crate::Interaction<Action = Action, Root = Root>,
    {
        let stage_interaction::Arg {
            interaction,
        } = arg;

        let state = State::new();

        let context = interaction::Context::new(&self.root, self.local_player_id);
        let mut interactor = Interactor::new(lookup, &self.reservation, context);
        
        interaction.apply(&mut interactor)
            .map_err(crate::TempError::discard)?;
        let expected_versions = interactor.expected_versions();
        let records = interactor.take_records();
        
        let interaction = interaction::Staged {
            interaction,
            expected_versions,
        };
        let pending_transaction_id = self.log.stage_pending(lookup, interaction, Transaction::new(records))
            .map_err(crate::TempError::discard)?;

        Ok(state.into_output(stage_interaction::Ret {
            pending_transaction_id,
        }))
    }

    pub fn apply_interaction<Lookup>(&mut self, _lookup: &mut Lookup, arg: apply_interaction::Arg)
        -> Result<Output<Interaction, GameOutcome, apply_interaction::Ret>, apply_interaction::Error>
    where
        Lookup: item::Lookup,
        Action: crate::Action<Lookup>,
    {
        let apply_interaction::Arg {
            pending_transaction_id,
        } = arg;

        let mut state = State::new();

        let interactions = self.log.apply_pending(pending_transaction_id)?;

        for interaction in interactions {
            state.push_signal(server::signal::ApplyInteraction {
                interaction,
                pending_transaction_id,
            });
        }

        Ok(state.into_output(apply_interaction::Ret { }))
    }

    pub fn revert_interaction<Lookup>(&mut self, lookup: &mut Lookup, arg: revert_interaction::Arg)
        -> Result<Output<Interaction, GameOutcome, revert_interaction::Ret>, revert_interaction::Error>
    where
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
    {
        let revert_interaction::Arg {
            pending_transaction_id,
        } = arg;

        let state = State::new();

        self.log.revert_pending(lookup, Some(pending_transaction_id))
            .map_err(crate::TempError::discard)?;

        Ok(state.into_output(revert_interaction::Ret { }))
    }

    pub fn signal<Lookup>(&mut self, lookup: &mut Lookup, arg: signal::Arg<Action, GameOutcome>)
        -> Result<Output<Interaction, GameOutcome, signal::Ret>, signal::Error<Lookup::Error, Action::Error>>
    where 
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
    {
        let signal::Arg {
            signal,
        } = arg;

        let mut state = State::new();

        let ret = match signal {
            signal::Internal::InteractionResult(interaction_result) => self.interaction_result(&mut state, lookup, interaction_result)?,
            signal::Internal::ConfirmedTransaction(confirmed_transaction) => self.confirmed_transaction(&mut state, lookup, confirmed_transaction)?,
            signal::Internal::EndGame(end_game) => self.end_game(&mut state, lookup, end_game)?,
        };

        Ok(state.into_output(ret))
    }

    fn interaction_result<Lookup>(&mut self, _state: &mut State<Interaction, GameOutcome>, lookup: &mut Lookup, interaction_result: signal::InteractionResult<Action>)
        -> Result<signal::Ret, signal::Error<Lookup::Error, Action::Error>>
    where 
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
    {
        let signal::InteractionResult {
            pending_transaction_id,
            confirmed_transaction_id,
        } = interaction_result;

        if let Some((confirmed_transaction_id, extra_records)) = confirmed_transaction_id {
            self.log.confirm_pending(lookup, pending_transaction_id, confirmed_transaction_id, &extra_records)
                .map_err(crate::TempError::discard)?;
        } else {
            self.log.revert_pending(lookup, None)
                .map_err(crate::TempError::discard)?;
        }

        Ok(signal::Ret::new())
    }

    fn confirmed_transaction<Lookup>(&mut self, _state: &mut State<Interaction, GameOutcome>, lookup: &mut Lookup, confirmed_transaction: signal::ConfirmedTransaction<Action>)
        -> Result<signal::Ret, signal::Error<Lookup::Error, Action::Error>>
    where 
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
    {
        let signal::ConfirmedTransaction {
            confirmed_transaction,
        } = confirmed_transaction;

        self.log.apply_confirmed(lookup, confirmed_transaction)
            .map_err(crate::TempError::discard)?;

        Ok(signal::Ret::new())
    }

    fn end_game<Lookup>(&mut self, state: &mut State<Interaction, GameOutcome>, lookup: &mut Lookup, end_game: signal::EndGame<GameOutcome>)
        -> Result<signal::Ret, signal::Error<Lookup::Error, Action::Error>>
    where 
        Lookup: item::Lookup,
        Action: crate::Action<Lookup, Undo = Action>,
    {
        let signal::EndGame {
            game_outcome,
        } = end_game;
        // The game is ending anyway, so if we become desynced, just ignore it
        let _ = self.log.revert_pending(lookup, None);

        state.events.push(event::GameComplete { game_outcome }.into());

        Ok(signal::Ret::new())
    }
}

// ???
// #[derive(Debug)]
// #[derive(thiserror::Error)]
// pub enum Error<LookupError, ActionsError> {
//     #[error(transparent)]
//     Lookup(LookupError),
//     #[error(transparent)]
//     Action(ActionsError),
//     // TODO
//     #[error("Other synchronization error")]
//     Other,
// }

// impl<LookupError, ActionsError> From<crate::TempError> for Error<LookupError, ActionsError> {
//     fn from(_value: crate::TempError) -> Self {
//         Self::Other
//     }
// }
