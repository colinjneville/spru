local is_deck = ...
local player_id = context.player
local p = context.root.players:get(player_id);

-- This should be the only place we *need* to check if it is our turn, as the fsm
-- should always be on ToDraw when it is not our turn
if context.root.current_turn.current ~= player_id then
    error("It is not this player's turn")
end

p.fsm:transition(player.Machine.draw())

if is_deck then
    output:enqueue_trigger(reaction.Trigger.draw_from_deck())
else
    local card = context.root.discard:pop_top()
    p.hand:push_top(card)
end
