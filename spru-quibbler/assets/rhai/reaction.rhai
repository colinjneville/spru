let trigger = args;

if trigger == type.Trigger.start_game() {
    if context.root.has_started {
        throw "The game has already started";
    }

    if context.root.players.count == 0 {
        throw "At least one player is required";
    }

    context.root.has_started = true;

    output.enqueue_trigger(type.Trigger.start_round());
} else if trigger == type.Trigger.start_round() {
    let hand_size = context.root.round.value + 3;
    let player_count = context.root.players.count;
    // TODO This needs to apply a non-method-created Action, which I 
    // don't think is currently possible
    // context.root.deck.initialize

    context.root.deck.shuffle();

    let cards = context.root.deck.pop_top_many(hand_size * player_count + 1);

    for player_root in context.root.players.players {
        let hand = [];

        for i in 0 .. hand_size {
            hand.push(cards.pop());
        }

        print(`${hand.len} cards to player`);

        player_root.hand.push_top_many(hand);
    }

    if cards.len != 1 {
        throw "Deck does not have enough cards";
    }

    context.root.discard.push_top(cards.pop());

    context.root.current_turn.position = context.root.current_dealer.position;

    context.root.current_dealer.rotate();
} else if trigger == type.Trigger.draw_from_deck() {
    let p = context.root.players.get(context.player);
    let card = context.root.deck.pop_top();
    p.hand.push_top(card);
} else if trigger == type.Trigger.play() {
    let round_end = for player_root in context.root.players.players {
        if player_root.played.value == () {
            break false;
        }
    } ?? true;

    if round_end {
        output.enqueue_trigger(type.Trigger.end_round());
    }
} else if trigger == type.Trigger.end_round() {
    let max_len = 0;
    let max_words = 0;
    let max_len_winner;
    let max_words_winner;

    for player_id in context.root.players.ids {
        let player_root = context.root.players.get(player_id);
        let this_max_len = player_root.played.value.max_word_len;
        
        if this_max_len > max_len {
            max_len = this_max_len;
            max_len_winner = player_id;
        } else if this_max_len == max_len {
            max_len_winner = ();
        }

        let this_max_words = player_root.played.value.word_count;
        if this_max_words > max_words {
            max_words = this_max_words;
            max_words_winner = player_id;
        } else if this_max_words == max_words {
            max_words_winner = ();
        }

        // Clear hand
        player_root.hand.clear();
        // Clear played cards
        player_root.played.value = ();
    }

    // Award 10 bonus points to winners of longest word/most words
    for winner_id in [max_len_winner, max_words_winner] {
        if winner_id != () {
            context.root.players.get(winner_id).score.add_checked(10);
        }
    }

    // Reset to plays being optional
    context.root.round_fsm.transition(type.RoundFsmInput.score());
    // Clear the discard pile
    context.root.discard.clear();

    // Game ends after 8 rounds
    if context.root.round == 7 {
        output.enqueue_trigger(type.Trigger.end_game());
    } else {
        context.root.round.add_checked(1);
        output.enqueue_trigger(type.Trigger.start_round());
    }
} else if trigger == type.Trigger.end_game() {
    let max_score = 0;
    let final_scores = [];

    for player_id in context.root.player_map.ids {
        let player_root = context.root.player_map.get(player_id);
        max_score = max_score.max(player_root.score.value);
        final_scores.push([player_id, player_root.score.value]);
    }

    let winners = [];
    for final_score in final_scores {
        let player_id = final_score[0];
        let score = final_score[1];

        if score == max_score {
            winners.push(player_id);
        }
    }

    // TODO
    type.GameOutcome.create(winners, final_scores)
} else {
    throw "Unknown trigger";
}