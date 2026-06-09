use spru::common::error::PseudoError as _;
use spru::{common::error::AnyError, interactor::with};
use spru::item::IdT;
use spru_util::{fsm, maybe, pile};
use tracing::instrument;

use crate::reaction;
use crate::script::Script;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Deck,
    Discard,
}

impl spru::Interaction for Draw {
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;

    #[instrument(skip_all, ret, err)]
    fn apply<'l, Storage>(
        &self,
        interactor: &mut spru::interaction::Interactor<Storage, Self>,
    ) -> spru::interaction::Result<()>
    where
        Storage: spru::item::Storage<State = crate::State>,
    {
        let player_id = interactor.context().player;

        with! { interactor =>
            let root = interactor.get_root()?;
            // This should be the only place we *need* to check if it is our turn, as the fsm
            // should always be on ToDraw when it is not our turn
            if ~[root.current_turn]?.current() != Some(&player_id) {
                return Err(AnyError::from_string("It is not this player's turn").into_error().into());
            }
            let players = ~[root.players]?;
            let player = players.get(player_id)?;
            let fsm = ~[player.fsm]?;

            let discard = ~[root.discard]?;
        }

        fsm.update(fsm::transition(crate::player::machine::Input::Draw));

        match self {
            Draw::Deck => {
                interactor.enqueue_trigger(reaction::Trigger::DrawFromDeck);
            }
            Draw::Discard => {
                let card = discard.top().expect("Discard cannot be empty");
                discard.update(pile::pop_top());

                interactor
                    .get(player.hand)?
                    .update(pile::push_top(card.clone()));
            }
        }

        Ok(())
    }
}

const SCRIPT: Script = crate::script::script!("rhai/draw.rhai");

pub fn new(is_deck: bool) -> super::RhaiInteraction<bool> {
    let language = spru_script::Rhai::<crate::Actions, crate::Lexicon>::default();
    super::RhaiInteraction::new(language, SCRIPT.get(), is_deck)
}