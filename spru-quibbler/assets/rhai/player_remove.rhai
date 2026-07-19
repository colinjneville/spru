let player_id = context.player;

let player_root = context.root.players.remove(player_id);
player_root.hand.destroy();
player_root.score.destroy();
player_root.fsm.destroy();
player_root.played.destroy();

let index = context.root.current_turn.items.index_of(player_id);
context.root.current_turn.remove(index);
let index = context.root.current_dealer.items.index_of(player_id);
context.root.current_dealer.remove(index);
