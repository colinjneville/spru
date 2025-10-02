use spru::{follow, item};
use spru::item::IdT;
use spru_util::fsm;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum DrawLocation {
    Deck,
    Discard,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Deck,
    Discard,
}

impl spru::Interaction for Draw {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;

    fn apply<'l, Lookup>(
        &self, 
        interactor: &mut super::Interactor<Lookup>, 
    ) 
        -> spru::interaction::Result<()> 
    where 
        Lookup: spru::item::Lookup<State = Self::State>,
    {
        let player_id = interactor.context().player;
        let root = interactor.get_root()?;
        let fsm = follow!(
            root => root.players,
            players => players
                .get(player_id)
                .ok_or(anyhow::anyhow!("Invalid player id"))
                .map_err(anyhow::Error::into_boxed_dyn_error)
                .map_err(spru::error::AnyError::new_boxed)
                .map_err(spru::action::Error::from)?
                .fsm
        )?;
        
        fsm.update(fsm::transition(crate::player::machine::Input::Draw));
        
        Ok(())
    }
}
