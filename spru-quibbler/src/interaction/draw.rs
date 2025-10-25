use spru::follow;
use spru::item::IdT;
use spru_util::{fsm, pile};
use tracing::instrument;

use crate::reaction;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Deck,
    Discard,
}

impl spru::Interaction for Draw {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;

    #[instrument(skip_all, ret, err)]
    fn apply<'l, Lookup>(
        &self,
        interactor: &mut super::Interactor<Lookup>,
    ) -> spru::interaction::Result<()>
    where
        Lookup: spru::item::Lookup<State = Self::State>,
    {
        let player_id = interactor.context().player;
        let root = interactor.get_root()?;
        // This should be the only place we *need* to check if it is our turn, as the fsm
        // should always be on ToDraw when it is not our turn
        interactor.get(root.current_turn)?.expect(&player_id)?;

        let players = follow!(root => root.players)?;
        let player = players.get(player_id)?;

        let fsm = interactor.get(player.fsm)?;

        fsm.update(fsm::transition(crate::player::machine::Input::Draw));

        match self {
            Draw::Deck => {
                interactor.enqueue_trigger(reaction::Trigger::DrawFromDeck);
            }
            Draw::Discard => {
                let discard = follow!(root => root.discard)?;

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
