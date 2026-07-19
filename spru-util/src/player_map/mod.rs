pub mod error;
use spru_script::script;

use std::marker::PhantomData;

use derive_where::derive_where;
use spru::{common::error::AnyResult, player};
use tagset::tagset;
use telety::telety;

use crate::cloned;

/// Stores some state per player
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[script(include = [Methods])]
pub struct PlayerMap<PlayerState> {
    map: Vec<Option<(player::Id, PlayerState)>>,
}

#[script(partial = Methods)]
impl<PlayerState> PlayerMap<PlayerState> 
where 
    PlayerState: Clone + 'static,
{
    #[create]
    fn dflt() -> cloned::Create<PlayerMap<PlayerState>> {
        create()
    }

    #[method]
    fn destroy(&self) -> ((), cloned::Destroy<PlayerMap<PlayerState>>) {
        ((), cloned::destroy())
    }

    #[get(name = count)]
    fn _count(&self) -> usize {
        self.count()
    }

    #[get]
    fn ids(&self) -> Vec<player::Id> {
        self.map
            .iter()
            .filter_map(|p| 
                p.as_ref().map(|(id, _state)| *id)
            ).collect()
    }

    #[get]
    fn players(&self) -> Vec<PlayerState> {
        self.map
            .iter()
            .filter_map(|p| 
                p.as_ref().map(|(_id, state)| state.clone())
            ).collect()
    }

    #[method(name = get)]
    fn _get(&self, player_id: player::Id) -> (Option<PlayerState>, ) {
        (self.get(player_id).ok().cloned(), )
    }

    #[method(name = insert)]
    fn _insert(&self, player_id: player::Id, player_state: PlayerState) -> ((), Insert<PlayerState>) {
        ((), insert(player_id, player_state))
    }

    #[method(name = remove)]
    fn _remove(&self, player_id: player::Id) -> (Option<PlayerState>, Remove<PlayerState>) {
        (self.get(player_id).ok().cloned(), remove(player_id))
    }
}

impl<PlayerState> PlayerMap<PlayerState> {
    pub fn count(&self) -> usize {
        self.map.iter().flatten().count()
    }

    pub fn get(&self, id: player::Id) -> Result<&PlayerState, error::PlayerDoesNotExist> {
        self.map
            .get(id.into_u32() as usize)
            .and_then(Option::as_ref)
            .map(|(_, state)| state)
            .ok_or(error::PlayerDoesNotExist::new(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (player::Id, &PlayerState)> {
        self.map.iter().flatten().map(|(id, state)| (*id, state))
    }
}

pub fn create<PlayerState>() -> Create<PlayerState> {
    cloned::create(PlayerMap { map: vec![] })
}

pub fn insert<PlayerState>(
    id: player::Id,
    player_state: PlayerState,
) -> Insert<PlayerState> {
    Insert { id, player_state }
}

pub fn remove<PlayerState>(id: player::Id) -> Remove<PlayerState> {
    Remove {
        id,
        _p: PhantomData,
    }
}

pub fn destroy<PlayerState>() -> Destroy<PlayerState> {
    cloned::destroy()
}

pub type Create<PlayerState> = cloned::Create<PlayerMap<PlayerState>>;

pub type Destroy<PlayerState> = cloned::Destroy<PlayerMap<PlayerState>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct Insert<PlayerState> {
    id: player::Id,
    player_state: PlayerState,
}

impl<PlayerState: Clone> spru::action::Update for Insert<PlayerState> {
    type T = PlayerMap<PlayerState>;
    type Undo = Remove<PlayerState>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.id.into_u32() as usize;
        while value.map.len() <= index {
            value.map.push(None);
        }

        match &mut value.map[index] {
            Some(_) => Err(error::PlayerAlreadyExists::new(self.id).into()),
            option @ None => {
                *option = Some((self.id, self.player_state.clone()));
                Ok(remove(self.id))
            }
        }
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct Remove<PlayerState> {
    id: player::Id,
    _p: PhantomData<PlayerState>,
}

impl<PlayerState> spru::action::Update for Remove<PlayerState> {
    type T = PlayerMap<PlayerState>;
    type Undo = Insert<PlayerState>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.id.into_u32() as usize;
        if let Some(player_state) = value.map.get_mut(index)
            && let Some((_, player_state)) = player_state.take()
        {
            return Ok(insert(self.id, player_state));
        }

        Err(error::PlayerDoesNotExist::new(self.id).into())
    }
}

#[telety(crate::player_map)]
#[tagset(Create<PlayerState>)]
#[tagset(Destroy<PlayerState>)]
#[tagset(Insert<PlayerState>)]
#[tagset(Remove<PlayerState>)]
#[tagset(reserved(..8))]
pub struct Actions<PlayerState>;
