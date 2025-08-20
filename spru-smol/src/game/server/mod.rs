use std::{any, collections::HashMap};

use spru::actions;
use spru_message::{payload, Message};

use crate::{lobby, Routed, router, Router};

pub struct Server<GameData, PlayerInit, PlayerData, Actions, Reaction, Payload> {
    router: Router<Payload>,
    router_to_player: HashMap<router::Id, spru::player::Id>,
    player_to_router: HashMap<spru::player::Id, router::Id>,
    server: spru::Server<Actions, GameData, PlayerInit, PlayerData, Reaction>,
}

impl<GameData, PlayerInit, PlayerData, Actions, Reaction, Payload> Server<GameData, PlayerInit, PlayerData, Actions, Reaction, Payload> {
    pub fn new<Lookup, GameInit, LobbyInfo, MemberInfo>(
        lookup: &mut Lookup, 
        game_init: GameInit, 
        player_init: PlayerInit, 
        reaction: Reaction,
        lobby_output: lobby::Output<LobbyInfo, MemberInfo>,
        router: Router<Payload>,
    ) -> Result<Self, NewError<Lookup::Error, GameInit::Error, PlayerInit::Error, Actions::Error>> 
    where 
        Lookup: spru::item::Lookup,
        GameInit: spru::Init<PlayerData, Lookup, In = LobbyInfo, Out = GameData, Action = Actions>,
        PlayerInit: spru::Init<PlayerData, Lookup, In = MemberInfo, Out = PlayerData, Action = Actions>,
        Actions: actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
    {
        let lobby::Output {
            lobby_info,
            members_info,
        } = lobby_output;

        let mut router_to_player = HashMap::new();
        let mut player_to_router = HashMap::new();

        let mut init_transactions = vec![];

        let mut server = spru::Server::new(lookup, game_init, lobby_info, player_init, reaction)?;
        for member_info in members_info {
            let Routed {
                client_id,
                value,
            } = member_info;

            let spru::server::AddPlayer {
                transaction,
                player_id,
            } = server.add_player(lookup, value)?;
            
            router_to_player.insert(client_id, player_id);
            player_to_router.insert(player_id, client_id);

            init_transactions.push(transaction);
        }

        let output = Self {
            router,
            router_to_player,
            player_to_router,
            server,
        };

        // TODO initial syncing...
        for init_transaction in init_transactions {
            for &router_id in output.router_to_player.keys() {

            }
        }
        
        Ok(output)
    }

    pub async fn run<Lookup, Interaction>(&mut self, lookup: &mut Lookup) 
        -> Result<Reaction::GameOutcome, crate::TempError> 
    where
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Reaction: spru::Reaction<Actions, PlayerData, Input = Interaction::Output, GameOutcome: Clone>,
        Payload: payload::Variant<spru::communication::Server<Interaction>> +
                 payload::Variant<spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>>,
        spru::communication::Server<Interaction>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>: serde::Serialize + Send + any::Any,
    {
        let game_outcome = loop {
            let directive = self.router.recv::<spru::communication::Server<Interaction>>().await
                .unwrap();
            
            let (responses, game_outcome) = self.process_directive(lookup, directive)?;
            for response in responses {
                self.router.send(response).await
                    .map_err(|_| crate::TempError)?;
            }

            // Game has completed
            if let Some(game_outcome) = game_outcome {
                break game_outcome;
            }
        };

        Ok(game_outcome)
    }

    pub fn run_pending<Lookup, Interaction>(&mut self, lookup: &mut Lookup) 
        -> Result<Option<Reaction::GameOutcome>, crate::TempError>
    where
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Reaction: spru::Reaction<Actions, PlayerData, Input = Interaction::Output, GameOutcome: Clone>,
        Payload: payload::Variant<spru::communication::Server<Interaction>> +
                 payload::Variant<spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>>,
        spru::communication::Server<Interaction>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>: serde::Serialize + Send + any::Any,
    {
        while let Some(result) = self.run_one::<_, Interaction>(lookup) { 
            if let Some(game_output) = result? {
                return Ok(Some(game_output));
            }
        }
        Ok(None)
    }

    pub fn run_one<Lookup, Interaction>(&mut self, lookup: &mut Lookup) 
        // TODO gross
        -> Option<Result<Option<Reaction::GameOutcome>, crate::TempError>>
    where
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Reaction: spru::Reaction<Actions, PlayerData, Input = Interaction::Output, GameOutcome: Clone>,
        Payload: payload::Variant<spru::communication::Server<Interaction>> +
                 payload::Variant<spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>>,
        spru::communication::Server<Interaction>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>: serde::Serialize + Send + any::Any,
    {
        match self.router.try_recv::<spru::communication::Server<Interaction>>() {
            Ok(message) => {
                match self.process_directive(lookup, message) {
                    Ok((messages, game_outcome)) => {
                        for message in messages {
                            if let Err(_e) = self.router.send_blocking(message){
                                return Some(Err(crate::TempError));
                            }
                        }
                        
                        Some(Ok(game_outcome))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
            // No pending directives
            Err(_) => None,
        }
    }

    fn process_directive<Lookup, Interaction>(&mut self, lookup: &mut Lookup, message: Routed<spru::communication::Server<Interaction>>)
        -> Result<(Vec<Routed<spru::communication::Client<PlayerData, Actions, Reaction::GameOutcome>>>, Option<Reaction::GameOutcome>), crate::TempError>
    where 
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Reaction: spru::Reaction<Actions, PlayerData, Input = Interaction::Output, GameOutcome: Clone>,
        Payload: payload::Variant<spru::communication::Server<Interaction>>,
    {
        let Routed { client_id, value } = message;

        let &player_id = self.router_to_player.get(&client_id)
            .ok_or(crate::TempError)?;

        match self.server.apply_signal(lookup, player_id, value) {
            Ok(output) => {
                let spru::communication::communication::Output {
                    outbound,
                    game_outcome,
                } = output;

                let mut messages = vec![];

                for (player_id, directive) in outbound.directives {
                    let &client_id = self.player_to_router.get(&player_id)
                        .ok_or(crate::TempError)?;

                    messages.push(Routed {
                        client_id,
                        value: directive,
                    })
                }

                Ok((messages, game_outcome))
            },
            Err(_) => Err(crate::TempError),
        }
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum NewError<LookupError, GameInitError, PlayerInitError, ActionsError> {
    NewServer(#[from] spru::server::NewError<LookupError, GameInitError>),
    AddPlayer(#[from] spru::server::AddPlayerError<LookupError, PlayerInitError, ActionsError>),
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum RunError<LookupError, ActionsError, InteractionError> {
    ApplyInteraction(#[from] spru::server::ApplyInteractionError<LookupError, ActionsError, InteractionError>),
}
