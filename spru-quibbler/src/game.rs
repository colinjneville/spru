use std::collections::HashMap;

use spru::item::IdT;
use spru_script::{Wrap, script};
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
        let deck = interactor.create(pile::create(card::all().into_iter().map(Wrap))).id();
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
#[script(include = [Impl])]
pub struct Root {
    #[get]
    pub deck: IdT<pile::Pile<Wrap<Card>>>,
    #[get]
    pub discard: IdT<pile::Pile<Wrap<Card>>>,
    #[get]
    pub round: IdT<counter::Counter<u32>>,
    #[get]
    pub round_fsm: IdT<fsm::Fsm<crate::round::machine::Impl>>,
    #[get]
    pub players: IdT<player_map::PlayerMap<Wrap<player::Root>>>,
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
        deck: IdT<pile::Pile<Wrap<Card>>>,
        discard: IdT<pile::Pile<Wrap<Card>>>,
        round: IdT<counter::Counter<u32>>,
        round_fsm: IdT<fsm::Fsm<crate::round::machine::Impl>>,
        players: IdT<player_map::PlayerMap<Wrap<player::Root>>>,
        current_turn: IdT<rotating::Rotating<spru::player::Id>>,
        current_dealer: IdT<rotating::Rotating<spru::player::Id>>,
    ) 
        -> cloned::Create<Root>
    {
        cloned::create(Self { 
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
pub struct Outcome {
    pub winners: Vec<spru::player::Id>,
    pub final_scores: HashMap<spru::player::Id, u32>,
}

#[script(state = false)]
impl Outcome {
    #[function]
    fn create(winners: Vec<spru::player::Id>, final_scores: HashMap<spru::player::Id, u32>) -> Wrap<Self> {
        Wrap::new(Self {
            winners,
            final_scores,
        })
    }
}

pub mod init {
    const SCRIPT: crate::script::Script = crate::script::script!("lua/game_init.lua");

    pub fn new() -> crate::GameInit {
        crate::GameInit::new(spru_script_lua::Lua::new(), SCRIPT.get(), ())
    }
}

