use spru_util::{counter, fsm, pile, player_map, rotating};
use tagset::tagset;

use crate::{data, game, hand, player, round};

#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(impl spru::State)]
#[tagset(game::Root)]
#[tagset(player_map::State<player::Root>)]
#[tagset(fsm::State<player::machine::Impl>)]
#[tagset(fsm::State<round::machine::Impl>)]
#[tagset(pile::State<data::Card>)]
#[tagset(hand::State)]
#[tagset(counter::State<u32>)]
#[tagset(rotating::State<spru::player::Id>)]
#[tagset(crate::Play)]
#[tagset(derive(Debug))]
pub struct State;