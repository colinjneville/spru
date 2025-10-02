// pub mod game_start;
// pub mod draw;
// pub mod play;

use std::collections::HashMap;

use rand::Rng as _;
use spru::{follow, item::IdT};
use spru_util::{counter, pile, verbatim};

pub(crate) type Interactor<'l, 'r> = spru::reaction::Interactor<'l, 'r, crate::State, crate::Actions, IdT<crate::game::Root>, Trigger, crate::game::Outcome>;

#[derive(Debug, Clone)]
pub enum Trigger {
    StartGame,
    StartRound,
    DrawDiscard,
    DrawDeck,
    EndRound,
    EndGame,
}

impl Trigger {
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

        Ok(())
    }

    fn start_round(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let players = follow!(root => root.players)?;

        // Reset deck and shuffle
        follow!(root => root.deck)?
            .update(crate::actions::InitializeDeck)
            .update(pile::shuffle(&mut rand::rng()));

        // Advance round counter
        follow!(root => root.round)?
            .update(counter::add_checked(1));

        for (player_id, player_data) in players.iter() {
            // let hand = players.follow(player_data.hand)?;
        }

        todo!();

        Ok(())
    }

    fn draw(interactor: &mut Interactor, from_deck: bool) 
        -> spru::action::Result<()> 
    {
        let player_id = interactor.context().player
            .expect("Must have a player context");
        let mut root = interactor.get_root()?;

        let pile = follow!(
            root => if from_deck { root.deck } else { root.discard })?;
            
        let card = pile.top()
            .expect("Pile cannot be empty");
        pile.update(pile::pop_top());

        follow!(
            root => root.players,
            players => players.expect_player(player_id).hand
        )?
            .update(pile::push_top(card.clone()));

        Ok(())
    }

    fn draw_discard(interactor: &mut Interactor) 
        -> spru::action::Result<()> 
    {
        Self::draw(interactor, false)
    }

    fn draw_deck(interactor: &mut Interactor) 
        -> spru::action::Result<()> 
    {
        Self::draw(interactor, true)
    }

    fn play(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let root = interactor.get_root()?;
        let players = follow!(root => root.players)?;
        let round_end = 'round_end: {
            for (player_id, player_root) in players.iter() {
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

    fn end_round(interactor: &mut Interactor)
        -> spru::action::Result<()> 
    {
        let mut root = interactor.get_root()?;
        let mut players = follow!(root => root.players)?;

        let mut max_len = 0;
        let mut max_len_winner = None;
        let mut max_words = 0;
        let mut max_words_winner = None;
        for (player_id, player_root) in players.iter() {
            let played = interactor.get(player_root.played)?;

            let mut this_max_len = played.max_word_len();
            if this_max_len > max_len {
                this_max_len = max_len;
                max_len_winner = Some(player_id);
            } else if this_max_len == max_len {
                max_len_winner = None;
            };

            let mut this_max_words = played.word_count();
            if this_max_words > max_words {
                this_max_words = max_words;
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

        let round = interactor.get(root.round)?;
        if *round.value() == 10 {
            interactor.enqueue_trigger(Trigger::EndGame);
        } else {
            interactor.enqueue_trigger(Trigger::StartRound);
        }

        Ok(())
    }

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
            Trigger::DrawDiscard => Trigger::draw_discard(interactor),
            Trigger::DrawDeck => Trigger::draw_deck(interactor),
            Trigger::EndRound => Trigger::end_round(interactor),
            Trigger::EndGame => Trigger::end_game(interactor),
        }
    }
}
