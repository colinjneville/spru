use spru::{follow, item::{self, IdT}};
use spru_util::{fsm, pile};

use crate::data;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Discard {
    discard: data::Card,
}

impl spru::Interaction for Discard {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = crate::reaction::Trigger;
    
    fn apply<Lookup>(
        &self,
        interactor: &mut spru::interaction::Interactor<Lookup, Self::Action, Self::Root, Self::Trigger>
    )
        -> spru::interaction::Result<()> 
    where 
        Lookup: spru::item::Lookup<State = Self::State>,
    {
        let player_id = interactor.context().player;
        let mut root = interactor.get_root()?;

        let mut players = follow!(
            root => root.players,
        )?;

        let player_root = players.expect_player(player_id);

        interactor.get(player_root.fsm)?
            .update(fsm::transition(crate::player::machine::Input::Discard));

        let hand = interactor.get(player_root.hand)?;
            
        let hand_index = hand.iter()
            .position(|i| i == &self.discard)
            .ok_or(crate::anyhow!("Card is not in hand"))?;
        hand
            .update(pile::remove(hand_index));
        follow!(root => root.discard)?
            .update(pile::push_top(self.discard.clone()));

        Ok(())
    }
}