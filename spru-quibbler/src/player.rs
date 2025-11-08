use std::fmt;

use rust_fsm::state_machine;
use spru::{common::error::PsuedoError as _, interactor::with, item::IdT};
use spru_util::{cloned, counter, fsm, pile, player_map, rotating};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub username: String,
    // ip...
}

impl Input {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Init;

impl spru::player::Init for Init {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type In = Input;

    fn initialize(
        &self,
        interactor: &mut spru::player::init::Interactor<Self>,
        input: Self::In,
    ) -> spru::player::init::Result<()> {
        let root = interactor.get_root()?;
        if root.has_started {
            return Err(spru::common::error::AnyError::from_string(
                "cannot add player to started game",
            )
            .into_error()
            .into());
        }

        let score = interactor.create(counter::create(0)).id();
        let hand = interactor.create(pile::default()).id();
        let fsm = interactor.create(fsm::default()).id();
        let played = interactor.create(cloned::default()).id();

        let player_id = interactor.context().player;

        with! { interactor =>
            let root = interactor.get_root()?;
            let current_turn = ~[root.current_turn]?;
            let current_dealer = ~[root.current_dealer]?;
            let players = ~[root.players]?;
        };

        current_turn.update(rotating::insert(current_turn.len(), player_id));
        current_dealer.update(rotating::insert(current_dealer.len(), player_id));

        players.update(player_map::add_player(
            player_id,
            Root {
                data: input,
                hand,
                score,
                fsm,
                played,
            },
        ));

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub data: Input,
    pub hand: IdT<pile::Pile<data::Card>>,
    pub score: IdT<counter::Counter<u32>>,
    pub fsm: IdT<fsm::Fsm<machine::Impl>>,
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

impl fmt::Display for machine::State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            machine::State::ToDiscard => write!(f, "Discard"),
            machine::State::ToDraw => write!(f, "Draw"),
            machine::State::ToPlay => write!(f, "Play"),
        }
    }
}

use crate::data;
