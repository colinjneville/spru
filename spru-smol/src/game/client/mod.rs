use std::any;

use spru::transaction;
use spru_message::{payload, Message};

use crate::{router, Routed};

pub struct Client<PlayerData, Actions, Payload> {
    client: spru::Client<PlayerData, Actions>,
    connection: router::Connection<Payload>,
}

impl<PlayerData, Actions, Payload> Client<PlayerData, Actions, Payload> {
    pub fn new<Lookup>(lookup: &mut Lookup, connection: router::Connection<Payload>) -> Self {
        let client = spru::Client::init(lookup, )
    }

    pub fn run_one<Lookup, Interaction, GameOutcome>(&mut self, lookup: &mut Lookup)
        -> Option<Result<Option<GameOutcome>, crate::TempError>>
    where 
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Payload: payload::Variant<spru::communication::Client<PlayerData, Actions, GameOutcome>> + 
                payload::Variant<spru::communication::Server<Interaction>>,
        spru::communication::Client<PlayerData, Actions, GameOutcome>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Server<Interaction>: serde::Serialize + Send + any::Any,
    {
        match self.connection.try_recv::<spru::communication::Client<PlayerData, Actions, GameOutcome>>() {
            Ok(message) => {
                match self.process_directive(lookup, message) {
                    Ok((messages, game_outcome)) => {
                        for message in messages {
                            if let Err(_e) = self.connection.send_blocking(message) {
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

    pub fn run_pending<Lookup, Interaction, GameOutcome>(&mut self, lookup: &mut Lookup)
        -> Result<Option<GameOutcome>, crate::TempError> 
    where 
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Payload: payload::Variant<spru::communication::Client<PlayerData, Actions, GameOutcome>> + 
                payload::Variant<spru::communication::Server<Interaction>>,
        spru::communication::Client<PlayerData, Actions, GameOutcome>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Server<Interaction>: serde::Serialize + Send + any::Any,
    {
        while let Some(result) = self.run_one::<_, Interaction, _>(lookup) { 
            if let Some(game_outcome) = result? {
                return Ok(Some(game_outcome));
            }
        }
        Ok(None)
    }

    pub async fn run<Lookup, Interaction, GameOutcome>(&mut self, lookup: &mut Lookup)
        -> Result<GameOutcome, crate::TempError>
    where 
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Payload: payload::Variant<spru::communication::Client<PlayerData, Actions, GameOutcome>> + 
                 payload::Variant<spru::communication::Server<Interaction>>,
        spru::communication::Client<PlayerData, Actions, GameOutcome>: serde::de::DeserializeOwned + Send + any::Any,
        spru::communication::Server<Interaction>: serde::Serialize + Send + any::Any,
    {
        let game_outcome = loop {
            let directive = self.connection.recv::<spru::communication::Client<PlayerData, Actions, GameOutcome>>().await
                .unwrap();
            
            let (responses, game_outcome) = self.process_directive(lookup, directive)?;
            for response in responses {
                self.connection.send(response).await
                    .map_err(|_| crate::TempError)?;
            }

            // Game has completed
            if let Some(game_outcome) = game_outcome {
                break game_outcome;
            }
        };

        Ok(game_outcome)
    }

    fn process_directive<Lookup, Interaction, GameOutcome>(&mut self, lookup: &mut Lookup, directive: spru::communication::Client<PlayerData, Actions, GameOutcome>)
        -> Result<(Vec<spru::communication::Server<Interaction>>, Option<GameOutcome>), crate::TempError>
    where 
        Lookup: spru::item::Lookup,
        Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
        Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
        Payload: payload::Variant<spru::communication::Client<PlayerData, Actions, GameOutcome>> + 
                 payload::Variant<spru::communication::Server<Interaction>>,
    {
        let spru::communication::communication::Output {
            outbound,
            game_outcome,
        } = self.client.signal::<_, Interaction, _>(lookup, directive)
            .map_err(|_| crate::TempError)?;

        Ok((outbound.directives, game_outcome))
    }

    // pub fn apply_local_interaction<Lookup, Interaction>(&mut self, lookup: &mut Lookup) 
    //     -> Result<ApplyLocalInteraction, >
    // where 
    //     Lookup: spru::item::Lookup,
    //     Actions: spru::actions::Apply<Lookup, Undo = Actions> + Send + Sync + 'static,
    //     Interaction: spru::Interaction<Actions, PlayerData> + serde::Serialize + serde::de::DeserializeOwned + Send + 'static,
    // {

    // }
}

// pub struct ApplyLocalInteraction {
//     pub local_id: transaction::Id,
// }