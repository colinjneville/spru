pub mod draw;
pub use draw::Draw;
pub mod discard;
pub use discard::Discard;
pub mod play;
pub use play::Play;
use spru::item::IdT;
use spru_script::Wrap;
use tagset::tagset;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Out of turn")]
    OutOfTurn,
    #[error("Invalid state")]
    InvalidState,
}

type LuaInteraction<Args> = spru_script::Interaction<
    crate::State, 
    crate::Actions, 
    IdT<crate::game::Root>, 
    Wrap<crate::reaction::Trigger>, 
    spru_script_lua::Lua<crate::State, crate::Actions>,
    Args,
>;

type RhaiInteraction<Args> = spru_script::Interaction<
    crate::State, 
    crate::Actions, 
    IdT<crate::game::Root>, 
    Wrap<crate::reaction::Trigger>, 
    spru_script_rhai::Rhai<crate::State, crate::Actions>,
    Args,
>;

#[tagset(derive(Debug, Clone))]
#[tagset(impl spru::Interaction {
    type State = crate::State;
    type Action = crate::Actions;
    type Root = IdT<crate::game::Root>;
    type Trigger = Wrap<crate::reaction::Trigger>;
})]
#[tagset(impl tagset::proxy::serde::Serialize)]
#[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
#[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
#[tagset(Draw)]
#[tagset(Play)]
#[tagset(Discard)]
#[tagset(LuaInteraction<Wrap<crate::data::Card>>)]
#[tagset(LuaInteraction<Option<Wrap<crate::Play>>>)]
#[tagset(LuaInteraction<bool>)]
#[tagset(RhaiInteraction<Wrap<crate::data::Card>>)]
#[tagset(RhaiInteraction<Option<Wrap<crate::Play>>>)]
#[tagset(RhaiInteraction<bool>)]
pub struct Interaction;

pub struct Output;
