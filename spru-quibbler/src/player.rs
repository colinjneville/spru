use std::fmt;

use rust_fsm::state_machine;
use spru::{common::error::PseudoError as _, interactor::with, item::IdT};
use spru_script::script;
use spru_util::{cloned, counter, fsm, pile, player_map, rotating, state_cell};

use crate::data;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[script(state = false)]
pub struct Input {
    #[get]
    pub username: String,
    // ip...
}

impl Input {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Init;

impl spru::player::Init for Init {
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
#[script(state = false, include = [Impl])]
pub struct Root {
    #[get]
    pub data: Input,
    #[get]
    pub hand: IdT<pile::Pile<data::Card>>,
    #[get]
    pub score: IdT<counter::Counter<u32>>,
    #[get]
    pub fsm: IdT<fsm::Fsm<machine::Impl>>,
    #[get]
    pub played: IdT<state_cell::StateCell<Option<crate::Play>>>,
}

#[script(state = false, partial = Impl)]
impl Root {
    #[function]
    fn create(
        data: Input, 
        hand: IdT<pile::Pile<data::Card>>,
        score: IdT<counter::Counter<u32>>,
        fsm: IdT<fsm::Fsm<machine::Impl>>,
        played: IdT<state_cell::StateCell<Option<crate::Play>>>,
    ) 
        -> Root
    {
        Self { data, hand, score, fsm, played, }
    }
}

state_machine! {
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    machine_internal(ToDraw)

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

impl Clone for machine::Output {
    fn clone(&self) -> Self {
        match *self { }
    }
}

pub mod machine {
    pub use super::machine_internal::*;

    #[spru_script::script(state = false, derive = [Eq])]
    impl Input {
        #[function]
        fn draw() -> Self {
            Self::Draw
        }

        #[function]
        fn discard() -> Self {
            Self::Discard
        }

        #[function]
        fn play() -> Self {
            Self::Play
        }

        #[function]
        fn pass() -> Self {
            Self::Pass
        }
    }
}

pub mod init {
    const SCRIPT: crate::script::Script = crate::script::script!("rhai/player_init.rhai");

    pub fn new() -> crate::PlayerInit {
        let language = crate::Language::default();
        crate::PlayerInit::new(language, SCRIPT.get())
    }
}

