use spru::{follow, item::IdT};
use spru_util::pile;

use crate::interaction::Draw;

impl spru::Reaction for Draw {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Input = crate::interaction::Draw;
    type GameOutcome = crate::game::Outcome;

    fn apply(&self, interactor: &mut super::Interactor, input: Self::Input) 
        -> Result<Option<Self::GameOutcome>, spru::reaction::Error>
    where 
        crate::Actions: spru::Action<spru::item::lookup::Canonical<crate::State>>,  
    {
        let player_id = interactor.player_context().unwrap();
        let root = interactor.get_root()?;
        let player_root = follow!(
            root => root.players,
            players => players.expect_player(player_id))?;

        let source_id = match self {
            Draw::Deck => root.deck,
            Draw::Discard => root.discard,
        };

        let source_pile = interactor.get(&source_id)?;
        let card = source_pile.top().expect("The deck has run out");
        follow!(player_root => player_root.hand)?
            .update(pile::push_top(card.clone()));
        source_pile.update(pile::pop_top());

        Ok(None)
    }
}