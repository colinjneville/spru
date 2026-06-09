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
        register!(registration, game::Root, game::Root as GameRoot);
        register!(registration, player_map::PlayerMap, player_map::PlayerMap<player::Root> as PlayerMap);
        register!(registration, fsm::Fsm, fsm::Fsm<player::machine::Impl> as PlayerFsm);
        register!(registration, fsm::Fsm, fsm::Fsm<round::machine::Impl> as RoundFsm);
        register!(registration, pile::Pile, pile::Pile<data::Card> as CardPile);
        register!(registration, counter::Counter, counter::Counter<u32> as CounterU32);
        register!(registration, rotating::Rotating, rotating::Rotating<spru::player::Id> as RotatingPlayer);
        register!(registration, state_cell::StateCell, state_cell::StateCell<Option<crate::Play>> as MaybePlay);

        // non-states
        register!(registration, player::Root, player::Root as PlayerRoot);
        register!(registration, player::Input, player::Input as PlayerInput);
        register!(registration, player::machine::Input, player::machine::Input as PlayerFsmInput);
        register!(registration, round::machine::Input, round::machine::Input as RoundFsmInput);
        register!(registration, data::Card, data::Card as Card);
        register!(registration, crate::Play, crate::Play as Play);
        register!(registration, reaction::Trigger, reaction::Trigger as Trigger);
        register!(registration, game::Outcome, game::Outcome as GameOutcome);
    }
}
