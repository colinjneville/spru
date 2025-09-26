#[cfg(feature = "bevy")]
mod bevy;
#[cfg(feature = "smol")]
mod smol;


use spru::item::IdT;
use tagset::tagset;

use spru_util::verbatim;

#[derive(Debug)]
pub struct LobbyInfo;

#[derive(Debug)]
pub struct MemberInfo(PlayerColor);

#[derive(Debug)]
pub struct Trigger(spru::player::Id);


#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameOutcome(pub spru::player::Id);

#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl<Lookup> spru::State<Lookup>)]
#[tagset(GameRoot)]
#[tagset(PlayerData)]
pub struct State;

#[tagset(derive(Clone))]
#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl spru::action::Base)]
#[tagset(impl<Lookup> spru::Action<Lookup>)]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(include(verbatim::Actions<GameRoot>))]
#[tagset(include(verbatim::Actions<PlayerData>))]
pub struct Actions;

#[derive(Debug, spru::FromInfallible)]
#[derive(thiserror::Error)]
#[error("{0}")]
pub struct Error(anyhow::Error);

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct Reaction;

impl spru::Reaction for Reaction {
    type State = State;
    type Action = Actions;
    type Root = IdT<GameRoot>;
    type Trigger = Trigger;
    type GameOutcome = GameOutcome;
    
    fn apply(&self, 
        interactor: &mut spru::reaction::Interactor<State, Actions, IdT<GameRoot>, Trigger, GameOutcome>, 
        input: Self::Trigger,
    ) 
        -> spru::action::Result<()>
    {
        interactor.set_game_outcome(GameOutcome(input.0));
        Ok(())
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

impl spru::player::Init for PlayerInit {
    type In = PlayerColor;
    type Root = IdT<GameRoot>;
    type State = State;
    type Action = Actions;

    fn initialize(
        &self, 
        interactor: &mut spru::player::init::Interactor<State, Actions, IdT<GameRoot>>, 
        input: Self::In
    ) 
        -> spru::player::init::Result<()> 
    {
        let player_root = interactor.create(verbatim::create(PlayerData {
            color: input,
        }));

        let mut game_root = interactor.get_root()?
            .clone();
        
        game_root.players.push(player_root.id());
        interactor.get_root()?
            .update(spru_util::verbatim::update(game_root));
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlayerData {
    color: PlayerColor,
}

#[derive(Debug, Default, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameRoot {
    players: Vec<IdT<PlayerData>>,
}

pub struct GameInit(pub LobbyInfo);

impl spru::game::Init for GameInit {
    type Root = IdT<GameRoot>;
    type State = State;
    type Action = Actions;

    fn initialize(
        self, 
        interactor: &mut spru::game::init::Interactor<State, Actions>
    ) 
        -> spru::game::init::Result<Self::Root> 
    {
        let root = interactor.create(spru_util::verbatim::create(GameRoot::default()));

        Ok(root.id())
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Interaction;
impl spru::Interaction for Interaction {
    type Action = Actions;
    type Root = IdT<GameRoot>;
    type Trigger = Trigger;

    fn apply<Lookup>(&self, interactor: &mut spru::interaction::Interactor<Lookup, Actions, IdT<GameRoot>, Trigger>) 
        -> spru::interaction::Result<()>
    where 
        Actions: spru::Action<Lookup>,
    {
        let root = interactor.get_root()?;
        interactor.enqueue_trigger(Trigger(interactor.context().player));
        Ok(())
    }
}
