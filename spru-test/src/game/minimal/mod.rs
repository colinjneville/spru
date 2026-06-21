use spru::{interactor::with, item::IdT};
use tagset::tagset;

use spru_util::{cloned, player_map};

#[derive(Debug)]
pub struct LobbyInfo;

#[derive(Debug)]
pub struct MemberInfo(pub PlayerColor);

#[derive(Debug)]
pub struct MyTrigger(spru::player::Id);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MyGameOutcome(pub spru::player::Id);

#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl spru::State)]
#[tagset(MyGameRoot)]
#[tagset(player_map::PlayerMap<PlayerData>)]
pub struct MyState;

#[tagset(derive(Clone))]
#[tagset(impl crate::proxy::std::fmt::Debug)]
#[tagset(impl spru::Action {
    type State = MyState;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(include(cloned::Actions<MyGameRoot>))]
#[tagset(include(player_map::Actions<PlayerData>))]
pub struct MyAction;

#[derive(Debug, spru_util::FromInfallible, thiserror::Error)]
#[error("{0}")]
pub struct Error(anyhow::Error);

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct MyReaction;

impl spru::Reaction for MyReaction {
    type Action = MyAction;
    type Root = IdT<MyGameRoot>;
    type Trigger = MyTrigger;
    type GameOutcome = MyGameOutcome;

    fn apply(
        &self,
        interactor: &mut spru::reaction::Interactor<Self>,
        input: Self::Trigger,
    ) -> spru::action::Result<()> {
        interactor.set_game_outcome(MyGameOutcome(input.0));
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
pub struct MyPlayerInit;

impl spru::player::Init for MyPlayerInit {
    type In = PlayerColor;
    type Root = IdT<MyGameRoot>;
    type Action = MyAction;

    fn initialize(
        &self,
        interactor: &mut spru::player::init::Interactor<Self>,
        input: Self::In,
    ) -> spru::player::init::Result<()> {
        let player_id = interactor.context().player;

        with! { interactor =>
            let root = interactor.get_root()?;
            let players = ~[root.players]?;
        };

        players.update(player_map::add_player(
            player_id,
            PlayerData { color: input },
        ));

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerData {
    color: PlayerColor,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MyGameRoot {
    players: IdT<player_map::PlayerMap<PlayerData>>,
}

pub struct GameInit(pub LobbyInfo);

impl spru::game::Init for GameInit {
    type Root = IdT<MyGameRoot>;
    type Action = MyAction;

    fn initialize(
        self,
        interactor: &mut spru::game::init::Interactor<Self>,
    ) -> spru::game::init::Result<Self::Root> {
        let players = interactor.create(player_map::create()).id();
        let root = interactor.create(spru_util::cloned::create(MyGameRoot { players }));

        Ok(root.id())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Interaction;
impl spru::Interaction for Interaction {
    type Action = MyAction;
    type Root = IdT<MyGameRoot>;
    type Trigger = MyTrigger;

    fn apply<Storage>(
        &self,
        interactor: &mut spru::interaction::Interactor<Storage, Self>,
    ) -> spru::interaction::Result<()>
    where
        Storage: spru::item::Storage<State = <MyAction as spru::Action>::State>,
    {
        let _root = interactor.get_root()?;
        interactor.enqueue_trigger(MyTrigger(interactor.context().player));
        Ok(())
    }
}

pub type MyServer = spru::server::Impl<Interaction, MyReaction, MyPlayerInit>;

pub type MyClient = spru::client::Impl<Interaction, MyGameOutcome>;

pub type MyCommon = <MyServer as spru::Server>::Common;
