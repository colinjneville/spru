let input = args;

if context.root.has_started {
    throw "cannot add player to started game";
}

let score = type.CounterU32.create(0);
let hand = type.CardPile.dflt();
let fsm = type.PlayerFsm.dflt();
let played = type.MaybePlay.create(());

context.root.current_turn.push(context.player);
context.root.current_dealer.push(context.player);

let player_root = type.PlayerRoot.create(input, hand, score, fsm, played);
context.root.players.add_player(context.player, player_root);
