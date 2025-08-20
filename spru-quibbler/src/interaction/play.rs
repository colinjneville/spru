use std::collections::HashMap;

use spru::{follow, item::IdT};
use spru_util::{counter, fsm, player_map};

use crate::{data::{self, Card}, interaction, player, round};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Play {
    play: Option<crate::Play>,
}

impl spru::Interaction for Play {
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;
    type Error = crate::Error;
    
    fn apply<Lookup>(&self, interactor: &mut super::Interactor<Lookup>)
         -> Result<(), spru::interaction::Error<Lookup::Error, Self::Error>>
    where 
        Lookup: spru::item::Lookup, 
        Self::Action: spru::Action<Lookup>,
    {
        let player_id = interactor.context().player;
        let root = interactor.get_root()?;
        let round_fsm = follow!(root => root.round_fsm)?;
        let players = follow!(root => root.players)?;
        let player = follow!(players => players.expect_player(player_id))?;
        let player_fsm = follow!(player => player.fsm)?;

        if let Some(play) = &self.play {
            round_fsm.update(fsm::transition(round::machine::Input::Play));
            player_fsm.update(fsm::transition(player::machine::Input::Play));

            let hand = follow!(player => player.hand)?;
            let mut remaining_cards = HashMap::<&data::Card, u8>::new();
            
            for card in hand.iter() {
                *remaining_cards.entry(card)
                    .or_insert(0) += 1;
            }

            for word in play.words() {
                if word.len() < 2 {
                    crate::bail!("Word must be 2+ cards");
                }

                let mut word_str = String::new();

                for card in word {
                    if remaining_cards.entry(card)
                        .or_insert(0)
                        .checked_sub(1).is_none() 
                    {
                        crate::bail!("Cards are not in hand");
                    }

                    word_str.push_str(card.face().letters);
                }

                if !wordnik_list::word_exists(&*word_str) {
                    crate::bail!("Word is not valid");
                }
            }

            // Add letter score with no bonuses
            follow!(player => player.score)?
                .update(counter::add_checked(play.base_score()));

            let mut round_complete = true;
            for (player_id, player_root) in players.iter() {
                if !follow!(player_root => player_root.played)?.is_played() {
                        round_complete = false;
                        break;
                }
            }
            
            'check_round_complete: {
                let max_len = 0;
                let max_len_winner = None;
                let max_words = 0;
                let max_words_winner = None;
                for (player_id, player_root) in players.iter() {
                    let played = root.follow(player_root.played)?;
                    
                    if !played.is_played() {
                        break 'check_round_complete;
                    }

                    let this_max_len = played.max_word_len();
                    if this_max_len > max_len {
                        this_max_len = max_len;
                        max_len_winner = Some(player_id);
                    } else if this_max_len == max_len {
                        max_len_winner = None;
                    };

                    let this_max_words = played.word_count();
                    if this_max_words > max_words {
                        this_max_words = max_words;
                        max_words_winner = Some(player_id);
                    } else if this_max_words == max_words {
                        max_words_winner = None;
                    };
                }

                // Award 10 bonus points to winners of longest word/most words
                for winner in [max_len_winner, max_words_winner] {
                    if let Some(winner) = winner {
                        follow!(
                            players => players.expect_player(winner),
                            winner => winner.score)?
                            .update(counter::add_checked(10));
                    }
                }
            };
            
        } else {
            round_fsm.update(fsm::transition(round::machine::Input::Pass));
            player_fsm.update(fsm::transition(player::machine::Input::Pass));
        }

        Ok(())
    }
}
