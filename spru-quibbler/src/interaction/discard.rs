use spru::{follow, item::{self, IdT}};
use spru_util::{fsm, pile};

use crate::data;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Discard {
    discard: data::Card,
}

impl spru::Interaction for Discard {
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;
    
    fn apply<Lookup>(
        &self,
        interactor: &mut spru::interaction::Interactor<Lookup, Self::Action, Self::Root, Self::Trigger>
    )
        -> spru::interaction::Result<()> 
    where 
        Self::Action: spru::Action<Lookup>,
    {
        let player_id = interactor.context().player;
        let mut root = interactor.get_root()?;

        let mut player_root = follow!(
            root => root.players,
            players => players.get(player_id).unwrap()
        )?;

        follow!(player_root => player_root.fsm)?
            .update(fsm::transition(crate::player::machine::Input::Discard));

        let hand = follow!(player_root => player_root.hand)?;
            
        let hand_index = hand.iter()
            .position(|i| i == &self.discard)
            .ok_or(anyhow::anyhow!("Card is not in hand"))?;
        hand
            .update(pile::remove(hand_index));
        follow!(root => root.discard)?
            .update(pile::push_top(self.discard.clone()));

        Ok(())
    }
}