use crate::{data, game, player, reaction, round};
use spru_util::{counter, fsm, pile, player_map, rotating, state_cell};

pub struct Lexicon;

impl spru_script::Lexicon for Lexicon {
    type Language = spru_script::Rhai<crate::Actions, Self>;

    fn register<Storage>(registration: &mut <Self::Language as spru_script::Language>::Registration<'_>)
    where
        Storage: spru::item::Storage<State = <<Self::Language as spru_script::Language>::Action as spru::Action>::State>,
    {
        macro_rules! register {
            ($registration:ident, $macro_path:path, $type_path:path $(as $rename_path:path)? $(,)?) => {
                $macro_path!(<Storage, crate::Actions> $registration => $type_path $(as $rename_path)?);
            };
        }

        // states
        register!(registration, game::register_Root, game::Root as GameRoot);
        register!(registration, player_map::register_PlayerMap, player_map::PlayerMap<player::Root> as PlayerMap);
        register!(registration, fsm::register_Fsm, fsm::Fsm<player::machine::Impl> as PlayerFsm);
        register!(registration, fsm::register_Fsm, fsm::Fsm<round::machine::Impl> as RoundFsm);
        register!(registration, pile::register_Pile, pile::Pile<data::Card> as CardPile);
        register!(registration, counter::register_Counter, counter::Counter<u32> as CounterU32);
        register!(registration, rotating::register_Rotating, rotating::Rotating<spru::player::Id> as RotatingPlayer);
        register!(registration, state_cell::register_StateCell, state_cell::StateCell<Option<crate::Play>> as MaybePlay);

        // non-states
        register!(registration, player::register_Root, player::Root as PlayerRoot);
        register!(registration, player::register_Input, player::Input as PlayerInput);
        register!(registration, player::machine::register_Input, player::machine::Input as PlayerFsmInput);
        register!(registration, round::machine::register_Input, round::machine::Input as RoundFsmInput);
        register!(registration, data::card::register_Card, data::Card as Card);
        register!(registration, crate::play::register_Play, crate::Play as Play);
        register!(registration, reaction::register_Trigger, reaction::Trigger as Trigger);
        register!(registration, game::register_Outcome, game::Outcome as GameOutcome);
    }
}
