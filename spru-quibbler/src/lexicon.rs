use crate::{data, game, player, reaction, round};
use spru_util::{counter, fsm, pile, player_map, rotating, state_cell};

pub struct Lexicon;

impl spru_script::StatelessLexicon for Lexicon {
    type Language = spru_script_rhai::Rhai;

    fn register_stateless(registration: &mut spru_script_rhai::Registration<'_>) {
        spru_script_rhai::rhai! { registration {
            player::register_Root           => player::Root as PlayerRoot;
            player::register_Data           => player::Data as PlayerData;
            player::machine::register_Input => player::machine::Input as PlayerFsmInput;
            round::machine::register_Input  => round::machine::Input as RoundFsmInput;
            data::card::register_Card       => data::Card as Card;
            crate::play::register_Play      => crate::Play as Play;
            reaction::register_Trigger      => reaction::Trigger as Trigger;
            game::register_Settings         => game::Settings as GameSettings;
            game::register_Outcome          => game::Outcome as GameOutcome;
        } };
    }
}

impl spru_script::Lexicon for Lexicon {
    type Action = crate::Actions;

    fn register_state<Storage>(registration: &mut spru_script_rhai::Registration<'_>)
    where
        Storage: spru::item::Storage<State = crate::State>,
    {
        spru_script_rhai::rhai! { <Storage, crate::Actions> registration {
            game::register_Root             => game::Root as GameRoot;
            player_map::register_PlayerMap  => player_map::PlayerMap<player::Root> as PlayerMap;
            fsm::register_Fsm               => fsm::Fsm<player::machine::Impl> as PlayerFsm;
            fsm::register_Fsm               => fsm::Fsm<round::machine::Impl> as RoundFsm;
            pile::register_Pile             => pile::Pile<data::Card> as CardPile;
            counter::register_Counter       => counter::Counter<u32> as CounterU32;
            rotating::register_Rotating     => rotating::Rotating<spru::player::Id> as RotatingPlayer;
            state_cell::register_StateCell  => state_cell::StateCell<Option<crate::Play>> as MaybePlay;
        } };
    }
}
