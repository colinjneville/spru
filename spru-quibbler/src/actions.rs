use spru::common::error::AnyResult;
use spru_script::Wrap;
use spru_util::{cloned, counter, fsm, pile, player_map, rotating, state_cell};
use tagset::tagset;

use crate::{data, game, player, round};

#[tagset(impl spru::Action {
    type State = crate::State;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(derive(Debug, Clone))]
#[tagset(include(cloned::Actions<game::Root>))]
#[tagset(include(cloned::Actions<state_cell::StateCell<Option<Wrap<crate::Play>>>>))]
#[tagset(include(fsm::Actions<player::machine::Impl>))]
#[tagset(include(fsm::Actions<round::machine::Impl>))]
#[tagset(include(pile::Actions<Wrap<data::Card>>))]
#[tagset(InitializeDeck)]
#[tagset(include(counter::Actions<u32>))]
#[tagset(include(player_map::Actions<Wrap<player::Root>>))]
#[tagset(include(rotating::Actions<spru::player::Id>))]
#[tagset(spru_util::maybe::Update<spru_util::fail::Update<pile::Pile<Wrap<data::Card>>>>)]
#[tagset(spru_util::fail::Update<pile::Pile<Wrap<data::Card>>>)]
pub struct Actions;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct InitializeDeck;

impl spru::action::Update for InitializeDeck {
    type T = pile::Pile<Wrap<data::Card>>;
    type Undo = cloned::Update<pile::Pile<Wrap<data::Card>>>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        use spru::action::Create as _;

        let (new_pile, _) = pile::create(data::card::all().into_iter().map(Wrap))
            .create()
            .expect("Infallible");

        let old_pile = std::mem::replace(value, new_pile);
        Ok(cloned::update(old_pile))
    }
}
