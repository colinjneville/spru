use spru::{follow, item::IdT};
use spru_util::{counter, fsm, pile, player_map, rotating, verbatim};
use rust_fsm::state_machine;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub username: String,
    // ip...
}

impl Input {
    pub fn new(username: String) -> Self {
        Self {
            username,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Init;

impl spru::player::Init for Init {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type In = Input;
    
    fn initialize(
        &self, 
        interactor: &mut spru::player::init::Interactor<Self::State, Self::Action, Self::Root>, 
        input: Self::In
    ) 
        -> spru::player::init::Result<()> 
    {
        let root = interactor.get_root()?;
        if root.has_started {
            // TODO Need a simpler path for string literal -> try-able error
            let e: spru::common::error::AnyError = spru::common::error::AnyError::new_boxed("Players can't join after the game has started".into());
            return Err(spru::common::error::PsuedoError::into_error(e).into());
        }

        let score = interactor.create(counter::create(0)).id();
        let hand = interactor.create(pile::default()).id();
        let fsm = interactor.create(fsm::default()).id();
        let played = interactor.create(verbatim::default()).id();

        let player_id = interactor.context().player;
        let root = interactor.get_root()?;
        let current_turn = follow!(root => root.current_turn)?;
        current_turn.update(rotating::insert(current_turn.len(), player_id));
        let current_dealer = follow!(root => root.current_dealer)?;
        current_dealer.update(rotating::insert(current_dealer.len(), player_id));

        follow!(root => root.players)?
            .update(player_map::add_player(player_id, Root {
                data: input,
                hand,
                score,
                fsm,
                played,
            }));

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub data: Input,
    pub hand: IdT<pile::State<data::Card>>,
    pub score: IdT<counter::State<u32>>,
    pub fsm: IdT<fsm::State<machine::Impl>>,
    pub played: IdT<crate::Play>,
}

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub machine(ToDraw)

    ToDraw(Draw) => ToDiscard,
    ToDiscard(Discard) => ToPlay,
    ToPlay => {
        Play => ToDraw,
        Pass => ToDraw,
    }
}

use crate::data;

