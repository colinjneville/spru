use std::collections::HashMap;

use spru::{interactor::with, item::IdT};
use spru_util::{counter, fsm, rotating, state_cell};
use tracing::instrument;

use crate::{data, player, reaction::Trigger, round, script::Script};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Play {
    play: Option<crate::Play>,
}

impl Play {
    pub fn pass() -> Self {
        Self { play: None }
    }
}

impl spru::Interaction for Play {
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
            let root = interactor.get_root::<crate::game::Root>()?;
            let round_fsm = ~[root.round_fsm]?;
            let players = ~[root.players]?;
            let player = players.get(player_id).unwrap();
            let player_fsm = ~[player.fsm]?;
        };

        if let Some(play) = &self.play {
            let play_kind = if play.is_full() {
                round::machine::Input::FullPlay
            } else {
                round::machine::Input::PartialPlay
            };

            round_fsm.update(fsm::transition(play_kind));
            player_fsm.update(fsm::transition(player::machine::Input::Play));

            let hand = interactor.get(player.hand)?;
            let mut remaining_cards = HashMap::<&data::Card, u8>::new();

            for card in hand.iter() {
                *remaining_cards.entry(card).or_insert(0) += 1;
            }

            for word in play.words() {
                if word.len() < 2 {
                    crate::bail!("Word must be 2+ cards");
                }

                let mut word_str = Vec::new();

                for card in word {
                    if remaining_cards
                        .entry(card)
                        .or_insert(0)
                        .checked_sub(1)
                        .is_none()
                    {
                        crate::bail!("Cards are not in hand");
                    }

                    word_str.extend_from_slice(card.face().letters);
                }

                word_str.make_ascii_lowercase();
                if !wordnik_list::word_exists(str::from_utf8(&word_str).unwrap()) {
                    crate::bail!("Word is not valid");
                }
            }

            // Add letter score with no bonuses
            interactor
                .get(player.score)?
                .update(counter::add_checked(play.base_score() as i32));

            tracing::info!(name: "hand_score", play = %play, score = play.base_score());

            interactor
                .get(player.played)?
                .update(state_cell::update(Some(play.clone())));

            interactor.enqueue_trigger(Trigger::Play);
        } else {
            round_fsm.update(fsm::transition(round::machine::Input::Pass));
            player_fsm.update(fsm::transition(player::machine::Input::Pass));
        }

        // Pass to next player
        interactor
            .get(root.current_turn)?
            .update(rotating::rotate(false));

        Ok(())
    }
}

const SCRIPT: Script = crate::script::script!("rhai/play.rhai");

pub fn new(play: Option<crate::Play>) -> super::RhaiInteraction<Option<crate::Play>> {
    let language = spru_script::Rhai::<crate::Actions, crate::Lexicon>::default();
    super::RhaiInteraction::new(language, SCRIPT.get(), play)
}
