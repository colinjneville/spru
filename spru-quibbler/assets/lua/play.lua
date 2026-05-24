local play = ...
local player_id = context.player
local p = context.root.players:get(player_id)

if play == nil then
    context.root.round_fsm:transition(round.Machine.pass())
    p.fsm:transition(player.Machine.pass())
else
    local play_kind
    if play.is_full then
        play_kind = round.Machine.full_play()
    else
        play_kind = round.Machine.partial_play()
    end

    context.root.round_fsm:transition(play_kind)
    p.fsm:transition(player.Machine.play())

    local remaining_cards = {}
    setmetatable(remaining_cards, {
        __index = function() return 0 end,
    })

    for _, card in ipairs(p.hand.items) do
        remaining_cards[card.letters] = remaining_cards[card.letters] + 1
    end

    for _, word in ipairs(play.words) do
        if #word < 2 then
            error("Word '" .. word[1].letters .. "' must be 2+ cards")
        end

        local word_str = ""

        for _, card in ipairs(word) do 
            remaining_cards[card.letters] = remaining_cards[card.letters] - 1
            if remaining_cards[card.letters] < 0 then
                error("Card '" .. card.letters .. "' is not in hand")
            end

            word_str = word_str .. card.letters
        end

        if not Play.check_word(word_str) then
            error("Word '" .. word_str .. "' is not valid")
        end
    end

    p.score:add_checked(play.base_score)

    p.played.value = play

    output:enqueue_trigger(reaction.Trigger.play())
end

context.root.current_turn:rotate(false)
