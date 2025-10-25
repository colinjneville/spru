use std::{collections::HashMap, mem};

use spru::{follow, item::IdT};
use spru_util::{counter, fsm, pile, rotating, verbatim};
use tracing::instrument;

use crate::{data, player, reaction::Trigger, round};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Play {
    play: Option<crate::Play>,
}

impl Play {
    pub fn pass() -> Self {
        Self { play: None }
    }

    pub fn parsed(hand: &pile::State<data::Card>, s: &[u8]) -> Result<Self, u8> {
        let mut remaining_cards = HashMap::new();
        for card in hand {
            *remaining_cards.entry(card).or_insert(0) += 1;
        }

        let mut words = vec![];
        let mut current_word = vec![];

        let mut iter = s.iter().copied().peekable();
        while let Some(first) = iter.next() {
            if first == b' ' {
                if !current_word.is_empty() {
                    words.push(mem::take(&mut current_word));
                }
            } else {
                let second = iter.peek().copied();
                let (first_card, second_card) = data::Card::get_matching(first, second);
                if let Some(second_card) = second_card
                    && let Some(card_count) = remaining_cards.get_mut(&second_card)
                    && *card_count > 0
                {
                    *card_count -= 1;
                    current_word.push(second_card.clone());
                    // Skip next letter, as we used a double letter card
                    iter.next();

                    continue;
                }

                if let Some(card_count) = remaining_cards.get_mut(&first_card)
                    && *card_count > 0
                {
                    *card_count -= 1;
                    current_word.push(first_card.clone());

                    continue;
                }

                // No cards for this letter(s)
                return Err(first);
            }
        }

        if !current_word.is_empty() {
            words.push(mem::take(&mut current_word));
        }

        let mut unused = vec![];
        for (card, count) in remaining_cards {
            for _ in 0..count {
                unused.push(card.clone());
            }
        }

        Ok(Self {
            play: Some(crate::Play::new(words, unused)),
        })
    }
}

impl spru::Interaction for Play {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;

    #[instrument(skip_all, ret, err)]
    fn apply<Lookup>(
        &self,
        interactor: &mut super::Interactor<Lookup>,
    ) -> spru::interaction::Result<()>
    where
        Lookup: spru::item::Lookup<State = Self::State>,
    {
        let player_id = interactor.context().player;

        let root = interactor.get_root::<crate::game::Root>()?;
        let round_fsm = follow!(root => root.round_fsm)?;
        let players = follow!(root => root.players)?;
        let player = players.expect_player(player_id);
        let player_fsm = interactor.get(player.fsm)?;

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

                let mut word_str = String::new();

                for card in word {
                    if remaining_cards
                        .entry(card)
                        .or_insert(0)
                        .checked_sub(1)
                        .is_none()
                    {
                        crate::bail!("Cards are not in hand");
                    }

                    word_str.push_str(card.face().letters);
                }

                word_str.make_ascii_lowercase();
                if !wordnik_list::word_exists(&word_str) {
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
                .update(verbatim::update(play.clone()));

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
