# spru architecture overview

spru works similarly to a specialized in-memory database, providing atomicity, consistency, and isolation. As spru is in-memory only, it does not natively provide durability.

## Server and clients
A spru game consists of 1 [Client] per player/spectator and 1 [Server]. The [Client]s can be in the same process as the [Server] (e.g. pass and play), networked on different machines (dedicated server), or a mix (1 player hosts for the other players). Communication is always between [Client] and [Server], never [Client] to [Client].    
[Server] to [Client] communication is abstracted as signals: any action (e.g. adding a player, a player interaction, etc.) on the server or a client may generate signals that must be passed to the [Server] or to a specified [Client]. 

## tagset


## Game Types and Traits
These are the traits and associated types you will need to implement to construct a game.  

### [State]
The most granular unit of mutation.  
A type implementing [State] usually maps to a 'mutable' physical component (a die, a counter, a dry-erase board, ...), a collection of something (a hand or deck of cards, a bag of tokens, ...), or a representation of the current state of the game (whose turn it is, what phase the game is in, ...). Things which are not mutable or created during the game (cards, tokens, ...) should generally not be [State]s.  
Multiple players can simultaneously interact with the game, as long as one does not mutate a [State] while another player has read or mutated it. More granular [State]s reduces this likelyhood.

### [item::IdT]
A weak reference to a [State].  
As [State]s are independent, one cannot contain another, they can only link indirectly.  

### [Action]
How [State]s are created, updated, and destroyed.  
[Action] is simply the combination of all possible [Subaction]s for the game.

### [Subaction]
How a [State] is created, updated or destroyed.  
A [Subaction] atomically alters a [State]. It takes the form of [action::Create], [action::Update] or [action::Destroy].  

### [action::Create]
Creates a new [State].  
If the [State] is serializable, usually [spru_util::verbatim::create] is sufficient.  

### [action::Update]
Updates an existing [State].  
If the update fails, it must leave the [State] as it was before the [action::Update] was attempted.  
If the [State] is serializable, [spru_util::verbatim::update] is useable, though custom [action::Update]s may be more efficient and provide more semantic information.    

### [action::Destroy]
Destroys an existing [State].  
If the [State] is serializable, usually [spru_util::verbatim::destroy] is sufficient.  

### [Interaction]
The way a player interacts with the game. An [Interaction] generally maps to a physical action, such as playing a card, moving a piece, or exchanging resources. [Interaction]s can also include non-physical actions, like choosing between a set of options.  

An [Interaction] generates a sequence of [Action]s. If any [Action] fails, the whole [Interaction] is reverted.  
[Interaction]s are first run on a client. The client can then attempt to commit it to the server or discard and revert it. If sent to the [Server], it will run and validate it as well. If accepted, it will be replicated to all [Client]s. Otherwise, the original [Client] will be instructed to rollback the [Interaction] locally.  
Notably, [Interaction]s should not involve any information hidden from the player, such as drawing the top card of a face-down deck to hand. In such a case, the [Interaction] should generate a [Trigger], the impetus for a [Server]-side [Reaction].  

### [Reaction::Trigger]
Input to start a [Reaction].  
Whenever an [Interaction] or [Reaction] raises a [Reaction::Trigger], the [Server] will run a [Reaction] with that [Reaction::Trigger].  

### [Reaction]
An automated action initiated by a [Trigger].  
[Reaction]s will often model upkeep that no particular player is responsible for, such as setting up for a new round, or shuffling the discard when the deck runs out.  
[Reaction]s only run on the [Server] and relay the outcome to all [Client]s. As it is [Server]-based, it has access to all [State]. [Reaction]s can also raise [Trigger]s, allowing you to chain multiple [Reaction]s as necessary.  
A [Reaction] can also determine the final outcome of a game. If it determines the game has ended, it outputs an object representing the game outcome, which will be send to all [Client]s.  

### [game::Init]
Sets the initial state of a game.  
This does any initial setup for the game. This runs before any players have joined, so any player-count specific initialization should be not be included here. [game::Init] returns a root object that all [Interaction]s and [Reaction]s will have access to. This object should act as the top of a hierarchy for all non-player-specific state.  

### [player::Init]
Runs initialization for each new player added to the game.  
This will run whenever you attempt to add a player to the game. If the player is allowed to join, this returns a root object for the player. Like the game root, this acts as the top of a hierarchy for all player-specific state, such as the player's hand and resources.  

## Infrastructure Types and Traits

### [Item]
Metadata wrapping for [State]s.  
These contain version information for the [State]s. You shouldn't need to deal with these unless you are writing an [item::Lookup] implementation.

### [item::Lookup]
Links [item::IdT]s to [State]s.  
Since [State]s are independent, they reference each other using [item::IdT]s. An [item::Lookup] finds the [State] for the given Id.

## Additional crates
The spru crate aims to be as general as possible and only implements the core [Server] and [Client]. Additional layers can be built on top to cover more responsibilities.

### spru-util
Mainly contains generally useful [State] and [Action] implementations. It also contains a minimal implementation for [item::Lookup], [spru_util::lookup::Standalone].  

### spru-bevy
Bevy integration. Contains plugins for servers, clients, and local transport. No networking support yet. [Item]s are created as components on bevy entities, allowing for using bevy's change detection.  
