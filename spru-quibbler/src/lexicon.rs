use crate::{data, game, player, reaction, round};
use spru_util::{counter, fsm, pile, player_map, rotating, state_cell};

pub struct Lexicon;

impl spru_script::StatelessLexicon for Lexicon {
    type Language = spru_script_rhai::Rhai;

    fn register_stateless(registration: &mut spru_script_rhai::Registration<'_>) {
        spru_script_rhai::rhai! { registration {
            player::register_Root           => player::Root as PlayerRoot;
            player::register_Input          => player::Input as PlayerInput;
            player::machine::register_Input => player::machine::Input as PlayerFsmInput;
            round::machine::register_Input  => round::machine::Input as RoundFsmInput;
            data::card::register_Card       => data::Card as Card;
            crate::play::register_Play      =>  crate::Play as Play;
            reaction::register_Trigger      => reaction::Trigger as Trigger;
            game::register_Outcome          => game::Outcome as GameOutcome;
        } };

        // macro_rules! register {
        //     ($registration:ident, $macro_path:path, $type_path:path $(as $rename_path:path)? $(,)?) => {
        //         $macro_path!($registration => $type_path $(as $rename_path)?);
        //     };
        // }

        // rhai! {
        //     player::register_Root => player::Root as PlayerRoot;
        // }
        // register!(registration, player::register_Root, player::Root as PlayerRoot);
        // register!(registration, player::register_Input, player::Input as PlayerInput);
        // register!(registration, player::machine::register_Input, player::machine::Input as PlayerFsmInput);
        // register!(registration, round::machine::register_Input, round::machine::Input as RoundFsmInput);
        // register!(registration, data::card::register_Card, data::Card as Card);
        // register!(registration, crate::play::register_Play, crate::Play as Play);
        // register!(registration, reaction::register_Trigger, reaction::Trigger as Trigger);
        // register!(registration, game::register_Outcome, game::Outcome as GameOutcome);
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
        // macro_rules! register {
        //     ($registration:ident, $macro_path:path, $type_path:path $(as $rename_path:path)? $(,)?) => {
        //         $macro_path!(<Storage, crate::Actions> $registration => $type_path $(as $rename_path)?);
        //     };
        // }

        // register!(registration, game::register_Root, game::Root as GameRoot);
        // register!(registration, player_map::register_PlayerMap, player_map::PlayerMap<player::Root> as PlayerMap);
        // register!(registration, fsm::register_Fsm, fsm::Fsm<player::machine::Impl> as PlayerFsm);
        // register!(registration, fsm::register_Fsm, fsm::Fsm<round::machine::Impl> as RoundFsm);
        // register!(registration, pile::register_Pile, pile::Pile<data::Card> as CardPile);
        // register!(registration, counter::register_Counter, counter::Counter<u32> as CounterU32);
        // register!(registration, rotating::register_Rotating, rotating::Rotating<spru::player::Id> as RotatingPlayer);
        // register!(registration, state_cell::register_StateCell, state_cell::StateCell<Option<crate::Play>> as MaybePlay);
    }
}
