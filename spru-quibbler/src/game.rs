use std::collections::HashMap;

use spru::item::IdT;
use spru_util::{counter, fsm, pile, player_map, rotating, verbatim};

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

    fn initialize(self, interactor: &mut spru::game::init::Interactor<Self>) -> spru::game::init::Result<Self::Root> {
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

        let root = interactor.create(verbatim::create(root)).id();
        Ok(root)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Root {
    pub deck: IdT<pile::State<Card>>,
    pub discard: IdT<pile::State<Card>>,
    pub round: IdT<counter::State<u32>>,
    pub round_fsm: IdT<fsm::State<crate::round::machine::Impl>>,
    pub players: IdT<player_map::State<player::Root>>,
    pub current_turn: IdT<rotating::State<spru::player::Id>>,
    pub current_dealer: IdT<rotating::State<spru::player::Id>>,

    pub has_started: bool,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub winners: Vec<spru::player::Id>,
    pub final_scores: HashMap<spru::player::Id, u32>,
}
