let is_deck = args;
let player_id = context.player;
let p = context.root.players.get(player_id).dynamic;

// This should be the only place we *need* to check if it is our turn, as the fsm
// should always be on ToDraw when it is not our turn
if context.root.current_turn.current.dynamic != player_id {
    throw "It is not this player's turn";
}

p.fsm.transition(type["player::Machine"].draw());

if is_deck {
    output.enqueue_trigger(type["reaction::Trigger"].draw_from_deck());
} else {
    let card = context.root.discard.pop_top().dynamic;
    p.hand.push_top(card);
}
