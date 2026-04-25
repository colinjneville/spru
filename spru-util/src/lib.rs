//! Common game items and utilities for use in spru games

pub mod bounds;
pub mod cloned;
pub mod counter;
pub mod die;
pub mod fsm;
pub mod pile;
pub mod player_map;
pub mod rotating;
pub mod scripting;
pub mod storage;
mod strictness;
pub use strictness::Strictness;

pub(crate) type Rng = rand_chacha::ChaCha8Rng;

pub use spru_macro::FromInfallible;
