use spru::common::error::AnyResult;
use spru_util::{counter, fsm, pile, player_map, rotating, verbatim};
use tagset::tagset;

use crate::{data, game, hand, player, round};

#[tagset(impl spru::Action {
    type State = crate::State;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(derive(Debug, Clone))]
#[tagset(include(verbatim::Actions<game::Root>))]
#[tagset(include(verbatim::Actions<crate::Play>))]
#[tagset(include(fsm::Actions<player::machine::Impl>))]
#[tagset(include(fsm::Actions<round::machine::Impl>))]
#[tagset(include(pile::Actions<data::Card>))]
#[tagset(InitializeDeck)]
#[tagset(include(hand::Actions))]
#[tagset(include(counter::Actions<u32>))]
#[tagset(include(player_map::Actions<player::Root>))]
#[tagset(include(rotating::Actions<spru::player::Id>))]
pub struct Actions;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct InitializeDeck;

impl spru::action::Update for InitializeDeck {
    type T = pile::State<data::Card>;
    type Undo = verbatim::Update<pile::State<data::Card>>;
    
    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        use spru::action::Create as _;

        let (new_pile, _) = pile::create(data::Card::all())
            .create()
            .expect("Infallible");
        
        let old_pile = std::mem::replace(value, new_pile);
        Ok(verbatim::update(old_pile))
    }
}
