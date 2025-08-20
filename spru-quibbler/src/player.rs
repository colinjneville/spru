use std::convert;

use spru::{follow, item::IdT};
use spru_util::{counter, fsm, pile, player_map, rotating, verbatim};
use rust_fsm::state_machine;

type Interactor<'l, 'r> = spru::player::init::Interactor<'l, 'r, crate::State, crate::Actions, IdT<crate::game::Root>>;

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

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Init;

impl spru::player::Init for Init {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type In = Input;
    type Error = crate::Error;
    
    fn initialize(
        &self, 
        interactor: &mut spru::player::init::Interactor<Self::State, Self::Action, Self::Root>, 
        input: Self::In
    ) 
        -> Result<(), spru::player::init::Error<Self::Error>> 
    {
        let score = interactor.create(counter::create(0))?;
        let hand = interactor.create(pile::default())?;
        let fsm = interactor.create(fsm::default())?;
        let played = interactor.create(verbatim::default())?;
        // let root = interactor.create(verbatim::create())?;

        let player_id = interactor.context().player;
        let mut root = interactor.get_root()?;
        let current_turn = follow!(root => root.current_turn)?;
        current_turn.update(rotating::insert(current_turn.position().unwrap(), player_id));

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

