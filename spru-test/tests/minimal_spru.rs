use std::collections::HashMap;

use spru_test::*;

use rand::seq::{IndexedRandom as _, };

#[test]
fn minimal_spru() {
    use spru_test::game::minimal;

    let mut rng = rand::rng();

    let mut runner = SyncRunner::<
        minimal::Server,
        minimal::Client,
        spru_util::lookup::Standalone<minimal::State>,
    >::new(minimal::GameInit(minimal::LobbyInfo), minimal::PlayerInit, minimal::Reaction).unwrap();

    for color in minimal::PLAYER_COLORS {
        runner.add_player(spru::server::add_player::Arg {
            init_input: color,
        }).unwrap();
    }
    
    let mut player_ids = vec![];
    let mut winner = None;

    let mut game_outcomes = HashMap::new();

    'game: loop {
        match runner.run_one().unwrap() {
            sync_runner::Run::Idle => break 'game,
            sync_runner::Run::Ran(events) => { 
                print!(".");
                for event in events {
                    match event {
                        Event::PlayerConfirmed(event::PlayerConfirmed { player_id }) => {
                            println!("joined: {}", player_id);
                            player_ids.push(player_id);
                            // Wait for all players to join
                            if player_ids.len() == minimal::PLAYER_COLORS.len() {
                                winner = player_ids.choose(&mut rng).copied();
                                println!("winner: {}", winner.unwrap());
                                
                                let interaction = minimal::Interaction;
                                runner.client_command(winner.unwrap(), spru::client::stage_interaction::Arg {
                                    interaction,
                                }).unwrap();
                            }
                        }
                        Event::InteractionStaged(event::InteractionStaged { player_id, pending_transaction_id }) => {
                            runner.client_command(player_id, spru::client::apply_interaction::Arg {
                                pending_transaction_id,
                            }).unwrap();
                        }
                        Event::ServerEvent(event::ServerEvent { event }) => match event {
                                spru::server::Event::GameComplete(game_complete) => {
                                    game_outcomes.insert(None, game_complete.game_outcome);
                                }
                                _ => { }
                            },
                        Event::ClientEvent(event::ClientEvent { player_id, event }) => match event {
                                spru::client::Event::GameComplete(game_complete) => {
                                    game_outcomes.insert(Some(player_id), game_complete.game_outcome);
                                }
                                _ => { }
                            },
                    }
                }
            }
        }
    }

    assert_eq!(game_outcomes.len(), minimal::PLAYER_COLORS.len() + 1);
    for game_outcome in game_outcomes.into_values() {
        assert_eq!(winner, Some(game_outcome.0));
    }
}