local trigger = ...

function start_game()
    if context.root.has_started then
        error("The game has already started")
    end

    context.root.has_started = true

    output:enqueue_trigger(reaction.Trigger.start_round())
end

function start_round()
    local hand_size = context.root.round.value + 3
    local player_count = context.root.players.count
    -- TODO This needs to apply a non-method-created Action, which I 
    -- don't think is currently possible
    -- context.root.deck.initialize

    context.root.deck:shuffle()

    local cards = context.root.deck:pop_top_many(hand_size * player_count + 1)

    for _, player_root in ipairs(context.root.players.players) do
        local hand = { }

        for i = 1, hand_size do
            table.insert(hand, table.remove(cards))
        end

        player_root.hand:push_top_many(hand)
    end

    if #cards ~= 1 then
        error("Deck does not have enough cards")
    end

    context.root.discard:push_top(table.remove(cards))

    context.root.current_turn.position = context.root.current_dealer.position

    context.root.current_dealer:rotate()
end

function draw_from_deck()
    local p = context.root.players:get(context.player)
    local card = context.root.deck:pop_top()
    p.hand:push_top(card)
end

function play()
    local round_end = true
    for _, player_root in ipairs(context.root.players.players) do
        if player_root.played.value == nil then
            round_end = false
            break
        end
    end

    if round_end then
        output:enqueue_trigger(reaction.Trigger.end_round())
    end
end

function end_round()
    local max_len, max_words = 0, 0
    local max_len_winner, max_words_winner

    for _, player_id in ipairs(context.root.players.ids) do
        local player_root = context.root.players:get(player_id)
        local this_max_len = player_root.played.value.max_word_len;
        
        if this_max_len > max_len then
            max_len = this_max_len
            max_len_winner = player_id
        elseif this_max_len == max_len then
            max_len_winner = nil
        end

        local this_max_words = player_root.played.value.word_count
        if this_max_words > max_words then
            max_words = this_max_words
            max_words_winner = player_id
        elseif this_max_words == max_words then
            max_words_winner = nil
        end

        -- Clear hand
        player_root.hand:clear()
        -- Clear played cards
        player_root.played.value = nil
    end

    -- Award 10 bonus points to winners of longest word/most words
    for _, winner_id in ipairs({ max_len_winner, max_words_winner }) do
        if winner_id ~= nil then
            context.root.players:get(winner_id).score:add_checked(10)
        end
    end

    -- Reset to plays being optional
    context.root.round_fsm:transition(round.Machine.score())
    -- Clear the discard pile
    context.root.discard:clear()

    -- Game ends after 7 rounds
    if context.root.round == 7 then
        output:enqueue_trigger(reaction.Trigger.end_game())
    else
        context.root.round:add_checked(1)
        output:enqueue_trigger(reaction.Trigger.start_round())
    end
end

function end_game()
    local max_score = 0
    local final_scores = { }

    for _, player_id in ipairs(context.root.players.ids) do
        local player_root = context.root.players:get(player_id)
        max_score = math.max(max_score, player_root.score.value)
        final_scores[player_id] = player_root.score.value
    end

    local winners = { }
    for player_id, score in pairs(final_scores) do
        if score == max_score then
            table.insert(winners, player_id)
        end
    end

    return game.Outcome.create(winners, final_scores)
end

if trigger == reaction.Trigger.start_game() then
    return start_game()
elseif trigger == reaction.Trigger.start_round() then
    return start_round()
elseif trigger == reaction.Trigger.draw_from_deck() then
    return draw_from_deck()
elseif trigger == reaction.Trigger.play() then
    return play()
elseif trigger == reaction.Trigger.end_round() then
    return end_round()
elseif trigger == reaction.Trigger.end_game() then
    return end_game()
else
    error("Unknown trigger")
end