#[cfg(feature = "bevy")]
mod bevy;
#[cfg(feature = "smol")]
mod smol;


use spru::{action, item::IdT};

#[derive(Debug)]
pub struct LobbyInfo;

#[derive(Debug)]
pub struct MemberInfo(PlayerColor);

#[derive(Debug)]
pub struct InteractionOutput(spru::player::Id);


#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameOutcome(pub spru::player::Id);

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::item::Catalog)]
pub enum ItemCatalog {
    GameRoot(GameRoot),
    PlayerRoot(PlayerRoot),
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Catalog)]
#[catalog(error = MyError)]
#[amass::amass_telety(crate::game::minimal)]
pub enum ActionCatalog {
    GameRoot(spru_util::action::verbatim::Catalog<GameRoot>),
    PlayerRoot(spru_util::action::verbatim::Catalog<PlayerRoot>),
}

#[derive(Debug)]
pub struct MyError;
impl From<std::convert::Infallible> for MyError {
    fn from(_value: std::convert::Infallible) -> Self {
        Self
    }
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error!")
    }
}

impl std::error::Error for MyError { }

#[derive(Debug)]
pub struct Reaction;

impl spru::interaction::Reaction<ItemCatalog, ActionCatalog, GameRoot> for Reaction {
    type Input = InteractionOutput;
    type GameOutcome = GameOutcome;
    
    fn apply(&self, 
        _interactor: &mut spru::interaction::Interactor<spru::item::lookup::Canonical<ItemCatalog>, ActionCatalog, GameRoot>, 
        input: Self::Input
    ) 
        -> Result<Option<Self::GameOutcome>, spru::interaction::reaction::Error>
    {
        Ok(Some(GameOutcome(input.0)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum PlayerColor {
    Red,
    Blue,
    Green,
    Yellow,
}

pub const PLAYER_COLORS: [PlayerColor; 4] = [PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green, PlayerColor::Yellow];

#[derive(Debug)]
pub struct PlayerInit;

impl spru::init::Base for PlayerInit {
    type In = PlayerColor;
    type Out = ();
    type Error = std::convert::Infallible;
}

impl spru::Init<ItemCatalog, ActionCatalog, GameRoot> for PlayerInit {
    fn initialize(&self, interactor: &mut spru::interaction::Interactor<spru::item::lookup::Canonical<ItemCatalog>, ActionCatalog, GameRoot>, input: Self::In) -> Result<Self::Out, spru::init::Error<Self::Error>> {
        let player_root = interactor.create(spru_util::action::verbatim::Create::new(PlayerRoot {
            color: input,
        })).map_err(spru::init::Error::Lookup)?;

        let mut game_root = interactor.root()
            .map_err(spru::init::Error::Lookup)?
            .get()
            .clone();
        
        game_root.players.push(player_root);
        interactor.update(spru_util::action::verbatim::Update::new(game_root), &interactor.root_id())
            .map_err(spru::init::Error::Lookup)?;        
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlayerRoot {
    color: PlayerColor,
}

#[derive(Debug, Default, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameRoot {
    players: Vec<IdT<PlayerRoot>>,
}

pub struct GameInit;
impl spru::init::Base for GameInit {
    type In = LobbyInfo;
    type Out = spru::item::IdT<GameRoot>;
    type Error = std::convert::Infallible;
}

impl spru::Init<ItemCatalog, ActionCatalog, GameRoot> for GameInit {
    fn initialize(&self, interactor: &mut spru::interaction::Interactor<spru::item::lookup::Canonical<ItemCatalog>, ActionCatalog, GameRoot>, _input: Self::In) -> Result<Self::Out, spru::init::Error<Self::Error>> {
        let root = interactor.create(spru_util::action::verbatim::Create::new(GameRoot::default()))
            .map_err(spru::init::Error::Lookup)?;

        Ok(root)
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Interaction;
impl spru::Interaction<ActionCatalog, GameRoot> for Interaction {
    type Output = InteractionOutput;
    type Error = std::convert::Infallible;

    fn apply<Lookup>(&self, _interactor: &mut spru::interaction::Interactor<Lookup, ActionCatalog, GameRoot>, player_id: spru::player::Id) 
        -> Result<Self::Output, spru::interaction::Error<Lookup::Error, Self::Error>>
    where 
        Lookup: spru::item::Lookup,
        ActionCatalog: spru::action::Catalog<Lookup>,
    {
        Ok(InteractionOutput(player_id))
    }
}


// #[cfg(test)]
// mod test {
//     use super::*;

//     use std::collections::{HashMap, VecDeque};

//     type Server = spru::Server<ItemCatalog, ActionCatalog, GameRoot, PlayerInit, Reaction>;
//     type Client = spru::Client<ActionCatalog, GameRoot, Interaction, GameOutcome>;

//     #[test]
//     fn run() {
//         let mut server = spru::Server::new(GameInit, LobbyInfo, PlayerInit, Reaction)
//             .expect("Server::new failed");

//         let mut clients = HashMap::new();

//         let mut server_signals = VecDeque::new();
//         let mut client_signals = VecDeque::new();

//         fn run_queues(
//             server: &mut Server,
//             clients: &mut HashMap<spru::player::Id, (Client, spru_util::lookup::Standalone)>,
//             server_signals: &mut VecDeque<(spru::player::Id, spru::server::signal::Arg<Interaction>)>, 
//             client_signals: &mut VecDeque<(spru::player::Id, spru::client::signal::Arg<ActionCatalog, GameOutcome>)>,
//         ) -> Result<Option<GameOutcome>, Box<dyn std::error::Error>> {
//             let mut outcome = None;
//             while !server_signals.is_empty() || !client_signals.is_empty() {
//                 if let Some((sender, server_directive)) = server_signals.pop_front() {
//                     let spru::server::Output {
//                         outbound,
//                         events,
//                         ret: spru::server::signal::Ret {

//                         },
//                     } = server.apply_signal(sender, server_directive)?;

                    

//                     client_signals.extend(outbound.signals.into_iter());
//                 }

//                 if let Some((recipient, client_directive)) = client_signals.pop_front() {
//                     let (client, lookup) = clients.get_mut(&recipient).unwrap();
//                     let spru::client::signal::Ret {
//                         outbound,
//                         game_outcome,
//                     } = client.signal(lookup, client_directive)
//                         .expect("Client::apply_directive failed");

//                     server_signals.extend(outbound.signals.into_iter().map(|d| (recipient, d)));
//                 }
//             }

//             Ok(outcome)
//         }

//         for i in 0..4 {
//             let new_player = Client::new_request(PLAYER_COLORS[i]);

//             let spru::server::add_player::Ret {
//                 client_init,
//                 outbound,
//                 player_id,
//             } = server.add_player(new_player)
//                 .expect("Server::add_player failed");

//             let mut lookup = spru_util::lookup::Standalone::new();

//             let client = spru::Client::init(&mut lookup, client_init)
//                 .expect("Client::new failed");

//             clients.insert(player_id, (client, lookup));

//             client_signals.extend(outbound.signals.into_iter());

//             let game_outcome = run_queues(&mut server, &mut clients, &mut server_signals, &mut client_signals)
//                 .unwrap();
//         }
//     }
// }
