use spru::item::IdT;
use tagset::tagset;

use spru_util::{player_map, verbatim};

#[derive(Debug)]
pub struct LobbyInfo;

#[derive(Debug)]
pub struct MemberInfo(pub PlayerColor);

#[derive(Debug)]
pub struct Trigger(spru::player::Id);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameOutcome(pub spru::player::Id);

#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl spru::State)]
#[tagset(GameRoot)]
#[tagset(player_map::State<PlayerData>)]
pub struct State;

#[tagset(derive(Clone))]
#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl spru::Action {
    type State = State;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(include(verbatim::Actions<GameRoot>))]
#[tagset(include(player_map::Actions<PlayerData>))]
pub struct Actions;

#[derive(Debug, spru::FromInfallible, thiserror::Error)]
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

    fn apply(
        &self,
        interactor: &mut spru::reaction::Interactor<Self>,
        input: Self::Trigger,
    ) -> spru::action::Result<()> {
        interactor.set_game_outcome(GameOutcome(input.0));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PlayerColor {
    Red,
    Blue,
    Green,
    Yellow,
}

pub const PLAYER_COLORS: [PlayerColor; 4] = [
    PlayerColor::Red,
    PlayerColor::Blue,
    PlayerColor::Green,
    PlayerColor::Yellow,
];

#[derive(Debug)]
pub struct PlayerInit;

impl spru::player::Init for PlayerInit {
    type In = PlayerColor;
    type Root = IdT<GameRoot>;
    type State = State;
    type Action = Actions;

    fn initialize(
        &self,
        interactor: &mut spru::player::init::Interactor<Self>,
        input: Self::In,
    ) -> spru::player::init::Result<()> {
        let player_id = interactor.context().player;
        let root = interactor.get_root()?;

        spru::follow!(root => root.players)?
            .update(player_map::add_player(player_id, PlayerData { color: input }));

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerData {
    color: PlayerColor,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameRoot {
    players: IdT<player_map::State<PlayerData>>,
}

pub struct GameInit(pub LobbyInfo);

impl spru::game::Init for GameInit {
    type Root = IdT<GameRoot>;
    type State = State;
    type Action = Actions;

    fn initialize(
        self,
        interactor: &mut spru::game::init::Interactor<Self>,
    ) -> spru::game::init::Result<Self::Root> {
        let players = interactor.create(player_map::create()).id();
        let root = interactor.create(spru_util::verbatim::create(GameRoot { players }));

        Ok(root.id())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Interaction;
impl spru::Interaction for Interaction {
    type State = State;
    type Action = Actions;
    type Root = IdT<GameRoot>;
    type Trigger = Trigger;

    fn apply<Lookup>(
        &self,
        interactor: &mut spru::interaction::Interactor<Lookup, Self>,
    ) -> spru::interaction::Result<()>
    where
        Lookup: spru::item::Lookup<State = Self::State>,
    {
        let _root = interactor.get_root()?;
        interactor.enqueue_trigger(Trigger(interactor.context().player));
        Ok(())
    }
}

pub type Server =
    spru::server::ServerImpl<Interaction, Reaction, PlayerInit>;

pub type Client = spru::client::ClientImpl<Interaction, GameOutcome>;

pub type Common = <Server as spru::Server>::Common;
