mod error;
pub use error::Error;

use std::marker::PhantomData;

use derive_where::derive_where;
use spru::{common::error::AnyResult, player};
use tagset::tagset;
use telety::telety;

use crate::verbatim;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State<PlayerState> {
    map: Vec<Option<(player::Id, PlayerState)>>,
}

impl<PlayerState> State<PlayerState> {
    pub fn count(&self) -> usize {
        self.map.iter().flatten().count()
    }

    pub fn get(&self, id: player::Id) -> Result<&PlayerState, Error> {
        self.map
            .get(id.into_u32() as usize)
            .and_then(Option::as_ref)
            .map(|(_, state)| state)
            .ok_or(Error::PlayerDoesNotExist(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (player::Id, &PlayerState)> {
        self.map.iter().flatten().map(|(id, state)| (*id, state))
    }

    pub fn expect_player(&self, id: player::Id) -> &PlayerState {
        self.get(id).unwrap()
    }
}

pub type Create<PlayerState> = verbatim::Create<State<PlayerState>>;

pub fn create<PlayerState>() -> Create<PlayerState> {
    verbatim::create(State { map: vec![] })
}

pub type Destroy<PlayerState> = verbatim::Destroy<State<PlayerState>>;

pub fn destroy<PlayerState>() -> Destroy<PlayerState> {
    verbatim::destroy()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct AddPlayer<PlayerState> {
    id: player::Id,
    player_state: PlayerState,
}

impl<PlayerState: Clone> spru::action::Update for AddPlayer<PlayerState> {
    type T = State<PlayerState>;
    type Undo = RemovePlayer<PlayerState>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.id.into_u32() as usize;
        while value.map.len() <= index {
            value.map.push(None);
        }

        match &mut value.map[index] {
            Some(_) => Err(Error::PlayerAlreadyExists(self.id).into()),
            option @ None => {
                *option = Some((self.id, self.player_state.clone()));
                Ok(remove_player(self.id))
            }
        }
    }
}

pub fn add_player<PlayerState>(
    id: player::Id,
    player_state: PlayerState,
) -> AddPlayer<PlayerState> {
    AddPlayer { id, player_state }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct RemovePlayer<PlayerState> {
    id: player::Id,
    _p: PhantomData<PlayerState>,
}

impl<PlayerState> spru::action::Update for RemovePlayer<PlayerState> {
    type T = State<PlayerState>;
    type Undo = AddPlayer<PlayerState>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.id.into_u32() as usize;
        if let Some(player_state) = value.map.get_mut(index)
            && let Some((_, player_state)) = player_state.take()
        {
            return Ok(add_player(self.id, player_state));
        }

        Err(Error::PlayerDoesNotExist(self.id).into())
    }
}

pub fn remove_player<PlayerState>(id: player::Id) -> RemovePlayer<PlayerState> {
    RemovePlayer {
        id,
        _p: PhantomData,
    }
}

#[telety(crate::player_map)]
#[tagset(Create<PlayerState>)]
#[tagset(Destroy<PlayerState>)]
#[tagset(AddPlayer<PlayerState>)]
#[tagset(RemovePlayer<PlayerState>)]
#[tagset(reserved(..8))]
pub struct Actions<PlayerState>;
