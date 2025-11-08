use spru_util::{counter, fsm, pile, player_map, rotating};
use tagset::tagset;

use crate::{data, game, player, round};

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(game::Root)]
#[tagset(player_map::PlayerMap<player::Root>)]
#[tagset(fsm::Fsm<player::machine::Impl>)]
#[tagset(fsm::Fsm<round::machine::Impl>)]
#[tagset(pile::Pile<data::Card>)]
#[tagset(counter::Counter<u32>)]
#[tagset(rotating::Rotating<spru::player::Id>)]
#[tagset(crate::Play)]
#[tagset(derive(Debug))]
pub struct State;
