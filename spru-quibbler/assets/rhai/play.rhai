let play = args.flatten();
let player_id = context.player;
let p = context.root.players.get(player_id);

if play == () {
    context.root.round_fsm.transition(type["RoundFsmInput"].pass());
    p.fsm.transition(type["PlayerFsmInput"].pass());
} else {
    let play_kind;
    if play.is_full {
        play_kind = type["RoundFsmInput"].full_play();
    } else {
        play_kind = type["RoundFsmInput"].partial_play()
    }

    context.root.round_fsm.transition(play_kind);
    p.fsm.transition(type["PlayerFsmInput"].play());

    let remaining_cards = #{ };

    for card in p.hand.items {
        remaining_cards[card.letters] = (remaining_cards[card.letters] ?? 0) + 1;
    }

    for word in play.words {
        let word = word.to_array();
        if word.len < 2 {
            throw `Word '${word[0].letters}' must be 2+ cards`;
        }

        let word_str = "";

        for card in word {
            remaining_cards[card.letters] = remaining_cards[card.letters] - 1;
            if remaining_cards[card.letters] < 0 {
                throw `Card '${card.letters}' is not in hand`;
            }

            word_str += card.letters;
        }

        if !type["Play"].check_word(word_str) {
            throw `Word '${word_str}' is not valid`;
        }
    }

    p.score.add_checked(play.base_score);

    p.played.value = play;

    output.enqueue_trigger(type["Trigger"].play());
}

context.root.current_turn.rotate();
