use std::collections::HashMap;

use spru::item::IdT;
use spru_util::{cloned, counter, fsm, pile, player_map, rotating};

use crate::{
    data::{Card, card},
    player,
};

// type Interactor<'l> = spru::game::init::Interactor<'l, crate::State, crate::Actions>;

#[derive(Debug)]
pub struct Start;

pub struct Init;

impl spru::game::Init for Init {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<Root>;

    fn initialize(
        self,
        interactor: &mut spru::game::init::Interactor<Self>,
    ) -> spru::game::init::Result<Self::Root> {
        let deck = interactor.create(pile::create(card::Card::all())).id();
        let discard = interactor.create(pile::create([])).id();
        let round = interactor.create(counter::create(0)).id();
        let round_fsm = interactor.create(fsm::default()).id();

        let players = interactor.create(player_map::create()).id();
        let current_turn = interactor.create(rotating::default()).id();
        let current_dealer = interactor.create(rotating::default()).id();

        let root = Root {
            deck,
            discard,
            round,
            round_fsm,
            players,
            current_turn,
            current_dealer,
            has_started: false,
        };

        let root = interactor.create(cloned::create(root)).id();
        Ok(root)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub deck: IdT<pile::Pile<Card>>,
    pub discard: IdT<pile::Pile<Card>>,
    pub round: IdT<counter::Counter<u32>>,
    pub round_fsm: IdT<fsm::Fsm<crate::round::machine::Impl>>,
    pub players: IdT<player_map::PlayerMap<player::Root>>,
    pub current_turn: IdT<rotating::Rotating<spru::player::Id>>,
    pub current_dealer: IdT<rotating::Rotating<spru::player::Id>>,

    pub has_started: bool,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub winners: Vec<spru::player::Id>,
    pub final_scores: HashMap<spru::player::Id, u32>,
}
