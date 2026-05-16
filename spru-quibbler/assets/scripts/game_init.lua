local deck = pile.Pile[Wrap[data.Card]].new(data.Card.all())
local discard = pile.Pile[Wrap[data.Card]].default()
local round_fsm = fsm.Fsm[round.machine.Impl].default()
local round = counter.Counter[u32].new(0)

local players = player_map.PlayerMap[Wrap[player.Root]].default()
local current_turn = rotating.Rotating[spru.player.Id].default()
local current_dealer = rotating.Rotating[spru.player.Id].default()

local root = game.Root.new(deck, discard, round, round_fsm, players, current_turn, current_dealer)

return root
