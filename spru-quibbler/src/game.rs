use std::collections::HashMap;

use spru::item::IdT;
use spru_script::script;
use spru_util::{cloned, counter, fsm, pile, player_map, rotating};

use crate::{
    data::{Card, card},
    player,
};

#[derive(Debug)]
pub struct Start;

pub struct Init;

impl spru::game::Init for Init {
    type Action = crate::Actions;
    type Root = IdT<Root>;

    fn initialize(
        self,
        interactor: &mut spru::game::init::Interactor<Self>,
    ) -> spru::game::init::Result<Self::Root> {
        let deck = interactor.create(pile::create(card::all().into_iter())).id();
        let discard = interactor.create(pile::create([])).id();
        let round = interactor.create(counter::create(0)).id();
        let round_fsm = interactor.create(fsm::default()).id();

        let players = interactor.create(player_map::create()).id();
        let current_turn = interactor.create(rotating::default()).id();
        let current_dealer = interactor.create(rotating::default()).id();

        let root = Root {
            settings: Settings { first_hand: 3, last_hand: 10, },
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
#[script(include = [Impl])]
pub struct Root {
    #[get]
    pub settings: Settings,
    #[get]
    pub deck: IdT<pile::Pile<Card>>,
    #[get]
    pub discard: IdT<pile::Pile<Card>>,
    #[get]
    pub round: IdT<counter::Counter<u32>>,
    #[get]
    pub round_fsm: IdT<fsm::Fsm<crate::round::machine::Impl>>,
    #[get]
    pub players: IdT<player_map::PlayerMap<player::Root>>,
    #[get]
    pub current_turn: IdT<rotating::Rotating<spru::player::Id>>,
    #[get]
    pub current_dealer: IdT<rotating::Rotating<spru::player::Id>>,
    #[get]
    #[set]
    pub has_started: bool,
}

#[script(partial = Impl)]
impl Root {
    #[create]
    fn create(
        settings: Settings,
        deck: IdT<pile::Pile<Card>>,
        discard: IdT<pile::Pile<Card>>,
        round: IdT<counter::Counter<u32>>,
        round_fsm: IdT<fsm::Fsm<crate::round::machine::Impl>>,
        players: IdT<player_map::PlayerMap<player::Root>>,
        current_turn: IdT<rotating::Rotating<spru::player::Id>>,
        current_dealer: IdT<rotating::Rotating<spru::player::Id>>,
    ) 
        -> cloned::Create<Root>
    {
        cloned::create(Self { 
            settings,
            deck, 
            discard, 
            round, 
            round_fsm, 
            players, 
            current_turn, 
            current_dealer, 
            has_started: false 
        })
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[script(state = false)]
pub struct Settings {
    #[get]
    pub first_hand: usize,
    #[get]
    pub last_hand: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self { 
            first_hand: 3, 
            last_hand: 10,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Outcome {
    pub winners: Vec<spru::player::Id>,
    pub final_scores: Vec<(spru::player::Id, String, u32)>,
}

#[script(state = false)]
impl Outcome {
    #[function]
    fn create(winners: Vec<spru::player::Id>) -> Self {
        Self {
            winners,
            final_scores: vec![],
        }
    }

    #[function]
    fn with_final_score(mut outcome: Self, player: spru::player::Id, name: String, score: u32) -> Self {
        outcome.final_scores.push((player, name, score));
        outcome
    }
}

pub mod init {
    const SCRIPT: crate::script::Script = crate::script::script!("rhai/game_init.rhai");

    pub fn new(settings: super::Settings) -> crate::GameInit {
        let language = crate::Language::default();
        crate::GameInit::new(language, SCRIPT.get(), settings)
    }
}

