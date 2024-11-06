use amass::amass_telety;
use spru_util::{action::verbatim, component::{counter, fsm, pile}};

use crate::{component::hand, data, player, game};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::Actions)]
#[actions(error = Error)]
#[amass_telety(crate::actions)]
pub enum Actions {
    GameRoot(verbatim::Actions<game::Root>),
    PlayerRoot(verbatim::Actions<player::Root>),
    PlayerState(fsm::Actions<player::Impl>),
    Pile(pile::Actions<data::Card>),
    Hand(hand::Actions),
    CounterU8(counter::Actions<u8>),
    CounterU16(counter::Actions<u16>),
}

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::actions)]
pub enum Error {
    CounterU8(counter::Error<u8>),
    CounterU16(counter::Error<u16>),
    Fsm(fsm::Error),
    Hand(hand::Error),
    Pile(pile::Error),
}
