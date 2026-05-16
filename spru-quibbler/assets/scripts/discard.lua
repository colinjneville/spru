local discarded = ...
local player_id = context.player
local p = context.root.players:get(player_id);

p.fsm:transition(player.Machine.discard())

local hand_index = nil
for key, value in ipairs(p.hand.items) do
    if value == discarded then
        hand_index = key - 1
        break
    end
end
if hand_index == nil then
    error("Card is not in hand")
else
    p.hand:remove(hand_index)
    context.root.discard:push_top(discarded)
end