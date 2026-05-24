local deck = pile.Pile[Wrap[data.Card]].create(data.Card.all())
local discard = pile.Pile[Wrap[data.Card]].dflt()
local round_fsm = fsm.Fsm[round.machine.Impl].dflt()
local round = counter.Counter[u32].create(0)

local players = player_map.PlayerMap[Wrap[player.Root]].dflt()
local current_turn = rotating.Rotating[spru.player.Id].dflt()
local current_dealer = rotating.Rotating[spru.player.Id].dflt()

local root = game.Root.create(deck, discard, round, round_fsm, players, current_turn, current_dealer)

return root
