use std::collections::HashMap;

use tracing::instrument;
use spru::{follow, item::IdT};
use spru_util::{counter, fsm, pile, rotating, verbatim};

use crate::round;

pub(crate) type Interactor<'l, 'r> = spru::reaction::Interactor<'l, 'r, crate::State, crate::Actions, IdT<crate::game::Root>, Trigger, crate::game::Outcome>;

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
    fn start_game(interactor: &mut Interactor) 
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let new_root = crate::game::Root {
            has_started: true,
            ..(*root).clone()
        };
        interactor.get_root()?
            .update(verbatim::update(new_root));

        interactor.enqueue_trigger(Trigger::StartRound);

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn start_round(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;

        let hand_size = *follow!(root => root.round)?.value() as usize + 3;

        // Reset deck and shuffle
        follow!(root => root.deck)?
            .update(crate::actions::InitializeDeck)
            .update(pile::shuffle(&mut rand::rng()));

        // Deck must be shuffled before dealing
        interactor.flush()?;

        let root = interactor.get_root()?;
        let deck = follow!(root => root.deck)?;

        let players = follow!(root => root.players)?;

        let mut hand_chunks = deck.chunks(hand_size);

        deck.update(pile::pop_top_many(hand_size * players.count() + 1));

        for (_player_id, player) in players.iter() {
            let hand = interactor.get(player.hand)?;

            let cards = hand_chunks.next()
                .expect("The deck must have enough cards")
                .to_vec();
            
            hand
                .update(pile::push_top_many(cards));
        }

        let discarded = hand_chunks.next()
            .expect("The deck must have enough cards")
            [0].clone();
        follow!(root => root.discard)?
            .update(pile::push_top(discarded));

        let current_dealer = follow!(root => root.current_dealer)?;
        let current_turn = follow!(root => root.current_turn)?;

        current_turn
            .update(rotating::set_position(current_dealer.position().unwrap()));

        current_dealer
            .update(rotating::rotate(false));

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn draw_from_deck(interactor: &mut Interactor) 
        -> spru::action::Result<()> 
    {
        let player_id = interactor.context().player
            .expect("Must have a player context");
        let root = interactor.get_root()?;

        let pile = follow!(root => root.deck)?;
            
        let card = pile.top()
            .expect("Pile cannot be empty");
        pile.update(pile::pop_top());

        follow!(
            root => root.players,
            players => players.get(player_id)?.hand
        )?
            .update(pile::push_top(card.clone()));

        Ok(())
    }

    #[instrument(skip_all, ret, err)]
    fn play(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let players = follow!(root => root.players)?;
        let round_end = 'round_end: {
            for (_player_id, player_root) in players.iter() {
                let played = root.follow(player_root.played)?;
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
    fn end_round(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let players = follow!(root => root.players)?;

        let mut max_len = 0;
        let mut max_len_winner = None;
        let mut max_words = 0;
        let mut max_words_winner = None;
        for (player_id, player_root) in players.iter() {
            let played = interactor.get(player_root.played)?;

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
            played.update(verbatim::update_default());
            // Clear hand
            root.follow(player_root.hand)?
                .update(pile::clear());
        }

        // Award 10 bonus points to winners of longest word/most words
        for winner in [max_len_winner, max_words_winner] {
            if let Some(winner) = winner {
                follow!(players => players.expect_player(winner).score)?
                    .update(counter::add_checked(10));
            }
        }

        // Reset to plays being optional
        interactor.get(root.round_fsm)?
            .update(fsm::transition(round::machine::Input::Score));

        // Clear the discard pile
        interactor.get(root.discard)?
            .update(pile::clear());

        let round = interactor.get(root.round)?;
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
    fn end_game(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let players = follow!(root => root.players)?;

        let mut max_score = 0;
        let mut final_scores = HashMap::new();
        for (player_id, player_root) in players.iter() {
            let player_score = root.follow(player_root.score)?;

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

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Reaction;

impl spru::Reaction for Reaction {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = Trigger;
    type GameOutcome = crate::game::Outcome;
    
    fn apply(&self, interactor: &mut self::Interactor, trigger: Self::Trigger) 
        -> spru::action::Result<()>
    {
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
