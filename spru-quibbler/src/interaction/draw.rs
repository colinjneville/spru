use spru::{follow, item};
use spru_bevy::item::IdT;
use spru_util::fsm;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum DrawLocation {
    Deck,
    Discard,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Deck,
    Discard,
}

impl spru::Interaction for Draw {
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;
    type Error = anyhow::Error;

    fn apply<'l, Lookup>(
        &self, 
        interactor: &mut super::Interactor<Lookup>, 
    ) 
        -> Result<(), spru::interaction::Error<Lookup::Error, Self::Error>> 
    where 
        Lookup: item::Lookup, 
        crate::Actions: spru::Action<Lookup>,
    {
        let player_id = interactor.context().player;
        let root = interactor.get_root()?;
        let player_root = follow!(
            root => root.players,
            players => players.get(player_id).ok_or(anyhow::anyhow!("Invalid player id"))?
            .get(player_id)
            )?;
        let player_root = interactor.get(&player_root)?;
        interactor.get(&player_root.state)?
            .update(fsm::transition(crate::player::machine::Input::Draw));
        
        Ok(())
    }
}
