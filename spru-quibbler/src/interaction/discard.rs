use spru::{interactor::with, item::IdT};
use spru_util::{fsm, pile};
use tracing::instrument;

use crate::{data, script::Script};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Discard {
    discard: data::Card,
}

impl Discard {
    pub fn new(discard: data::Card) -> Self {
        Self { discard }
    }
}

impl spru::Interaction for Discard {
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;

    #[instrument(skip_all, ret, err)]
    fn apply<Storage>(
        &self,
        interactor: &mut spru::interaction::Interactor<Storage, Self>,
    ) -> spru::interaction::Result<()>
    where
        Storage: spru::item::Storage<State = crate::State>,
    {
        let player_id = interactor.context().player;

        with! { interactor =>
            let root = interactor.get_root()?;
            let players = ~[root.players]?;
            let player_fsm = ~[players.get(player_id).unwrap().fsm]?;
            let hand = ~[players.get(player_id).unwrap().hand]?;
            let discard = ~[root.discard]?;
        };

        player_fsm.update(fsm::transition(crate::player::machine::Input::Discard));

        let hand_index = hand
            .iter()
            .position(|i| *i == self.discard)
            .ok_or(crate::anyhow!("Card is not in hand"))?;
        hand.update(pile::remove(hand_index));
        discard.update(pile::push_top(self.discard.clone()));

        Ok(())
    }
}

const SCRIPT: Script = crate::script::script!("rhai/discard.rhai");

pub fn new(discard: data::Card) -> super::RhaiInteraction<data::Card> {
    let language = crate::Language::default();
    super::RhaiInteraction::new(language, SCRIPT.get(), discard)
}

