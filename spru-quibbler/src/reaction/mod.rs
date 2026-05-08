use std::collections::HashMap;

use spru::{common::error::{AnyError, PseudoError as _}, interactor::with, item::IdT};
use spru_util::{cloned, counter, fsm, pile, rotating};
use tracing::instrument;

use crate::round;

type Interactor<'l, 'r> = spru::reaction::Interactor<'l, 'r, Reaction>;

#[derive(Debug, Clone)]
pub enum Trigger {
    StartGame,
    StartRound,
    DrawFromDeck,
    Play,
    EndRound,
    EndGame,
}

impl Trigger {
    #[instrument(skip_all, ret, err)]
    fn start_game(interactor: &mut Interactor) -> spru::action::Result<()> {
        let root = interactor.get_root()?;
        if root.has_started {
            Err(AnyError::from_string("The game has already started").into_error())?;
        }

        let new_root = crate::game::Root {
            has_started: true,
            ..(*root).clone()
        };
        interactor.get_root()?.update(cloned::update(new_root));

        interactor.enqueue_trigger(Trigger::StartRound);

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn start_round(interactor: &mut Interactor) -> spru::action::Result<()> {
        with! { interactor =>
            let root = interactor.get_root()?;
            let hand_size = *~[root.round]?.value() as usize + 3;
            let deck = ~[root.deck]?;
        };

        // Reset deck and shuffle
        deck.update(crate::actions::InitializeDeck)
            .update(pile::shuffle(&mut rand::rng()));

        // Deck must be shuffled before dealing
        interactor.flush()?;

        with! { interactor =>
            let root = interactor.get_root()?;
            let deck = ~[root.deck]?;
            let discard = ~[root.discard]?;
            let players = ~[root.players]?;
            let current_dealer = ~[root.current_dealer]?;
            let current_turn = ~[root.current_turn]?;
        };

        let mut hand_chunks = deck.chunks(hand_size);

        deck.update(pile::pop_top_many(hand_size * players.count() + 1));

        for (_player_id, player) in players.iter() {
            with! { interactor =>
                let hand = ~[player.hand]?;
            };

            let cards = hand_chunks
                .next()
                .expect("The deck must have enough cards")
                .to_vec();

            hand.update(pile::push_top_many(cards));
        }

        let discarded = hand_chunks.next().expect("The deck must have enough cards")[0].clone();
        discard.update(pile::push_top(discarded));

        current_turn.update(rotating::set_position(current_dealer.position().unwrap()));

        current_dealer.update(rotating::rotate(false));

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn draw_from_deck(interactor: &mut Interactor) -> spru::action::Result<()> {
        let player_id = interactor
            .context()
            .player
            .expect("Must have a player context");

        with! { interactor =>
            let root = interactor.get_root()?;
            let pile = ~[root.deck]?;
            let hand = ~[~[root.players]?.get(player_id)?.hand]?;
        };

        let card = pile.top().expect("Pile cannot be empty");
        pile.update(pile::pop_top());

        hand.update(pile::push_top(card.clone()));

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn play(interactor: &mut Interactor) -> spru::action::Result<()> {
        with! { interactor =>
            let root = interactor.get_root()?;
            let players = ~[root.players]?;
        };

        let round_end = 'round_end: {
            for (_player_id, player_root) in players.iter() {
                with! { interactor =>
                    let played = ~[player_root.played]?;
                }

                if !played.is_played() {
                    break 'round_end false;
                }
            }
            true
        };

        if round_end {
            interactor.enqueue_trigger(Trigger::EndRound);
        }

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn end_round(interactor: &mut Interactor) -> spru::action::Result<()> {
        with! { interactor =>
            let root = interactor.get_root()?;
            let players = ~[root.players]?;
            let discard = ~[root.discard]?;
            let round_fsm = ~[root.round_fsm]?;
            let round = ~[root.round]?;
        };

        let mut max_len = 0;
        let mut max_len_winner = None;
        let mut max_words = 0;
        let mut max_words_winner = None;
        for (player_id, player_root) in players.iter() {
            with! { interactor =>
                let played = ~[player_root.played]?;
                let hand = ~[player_root.hand]?;
            };

            let this_max_len = played.max_word_len();
            if this_max_len > max_len {
                max_len = this_max_len;
                max_len_winner = Some(player_id);
            } else if this_max_len == max_len {
                max_len_winner = None;
            };

            let this_max_words = played.word_count();
            if this_max_words > max_words {
                max_words = this_max_words;
                max_words_winner = Some(player_id);
            } else if this_max_words == max_words {
                max_words_winner = None;
            };

            // Clear played cards
            played.update(cloned::update_default());
            // Clear hand
            hand.update(pile::clear());
        }

        // Award 10 bonus points to winners of longest word/most words
        for winner in [max_len_winner, max_words_winner].into_iter().flatten() {
            with! { interactor =>
                let score = ~[players.get(winner).unwrap().score]?;
            };

            score.update(counter::add_checked(10));
        }

        // Reset to plays being optional
        round_fsm.update(fsm::transition(round::machine::Input::Score));

        // Clear the discard pile
        discard.update(pile::clear());

        if *round.value() == 7 {
            interactor.enqueue_trigger(Trigger::EndGame);
        } else {
            // Advance round counter
            round.update(counter::add_checked(1));

            interactor.enqueue_trigger(Trigger::StartRound);
        }

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn end_game(interactor: &mut Interactor) -> spru::action::Result<()> {
        with! { interactor =>
            let root = interactor.get_root()?;
            let players = ~[root.players]?;
        };

        let mut max_score = 0;
        let mut final_scores = HashMap::new();
        for (player_id, player_root) in players.iter() {
            with! { interactor =>
                let player_score = ~[player_root.score]?;
            };

            max_score = max_score.max(*player_score.value());
            final_scores.insert(player_id, *player_score.value());
        }

        let mut winners = vec![];
        for (&player_id, &score) in &final_scores {
            if score == max_score {
                winners.push(player_id);
            }
        }

        interactor.set_game_outcome(crate::game::Outcome {
            winners,
            final_scores,
        });

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reaction;

impl spru::Reaction for Reaction {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = Trigger;
    type GameOutcome = crate::game::Outcome;

    fn apply(
        &self,
        interactor: &mut spru::reaction::Interactor<Self>,
        trigger: Self::Trigger,
    ) -> spru::action::Result<()> {
        match trigger {
            Trigger::StartGame => Trigger::start_game(interactor),
            Trigger::StartRound => Trigger::start_round(interactor),
            Trigger::DrawFromDeck => Trigger::draw_from_deck(interactor),
            Trigger::Play => Trigger::play(interactor),
            Trigger::EndRound => Trigger::end_round(interactor),
            Trigger::EndGame => Trigger::end_game(interactor),
        }
    }
}
