local input = ...

if context.root.has_started then
    error("cannot add player to started game")
end

local score = counter.Counter[u32].create(0)
local hand = pile.Pile[Wrap[data.Card]].dflt()
local fsm = fsm.Fsm[player.machine.Impl].dflt()
local played = state_cell.StateCell[Option[Wrap[crate.Play]]].create(nil)

context.root.current_turn:push(context.player)
context.root.current_dealer:push(context.player)

local player_root = player.Root.create(input, hand, score, fsm, played)
context.root.players:add_player(context.player, player_root)
