let settings = args;
let deck = type.CardPile.create(type.Card.from_array(type.Card.all()));
let discard = type.CardPile.dflt();
let round_fsm = type.RoundFsm.dflt();
let round = type.CounterU32.create(0);

let players = type.PlayerMap.dflt();
let current_turn = type.RotatingPlayer.dflt();
let current_dealer = type.RotatingPlayer.dflt();

let root = type.GameRoot.create(settings, deck, discard, round, round_fsm, players, current_turn, current_dealer);

root