let discarded = args;
let player_id = context.player;
let p = context.root.players.get(player_id);

p.fsm.transition(type.PlayerFsmInput.discard());

let hand_index;
let hand_index = p.hand.items.index_of(discarded);

if hand_index == -1 {
    throw "Card is not in hand";
} else {
    p.hand.remove(hand_index);
    context.root.discard.push_top(discarded);
}