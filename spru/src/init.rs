
use std::marker::PhantomData;

use crate::{action, item, player, reaction, Interactor};

// pub trait Init {
//     type In;
//     type Out;
//     type Error;
//     type State: crate::State<item::lookup::Canonical<Self::State>>;
//     type Action: crate::Action<item::lookup::Canonical<Self::State>>;
//     type Context<'r>;

//     fn initialize(&self, interactor: &mut Interactor<item::lookup::Canonical<Self::State>, Self::Action, Self::Context<'_>>, input: Self::In) 
//         -> Result<Self::Out, Error<Self::Error>>;
// }

// #[derive(Debug)]
// #[non_exhaustive]
// pub struct GameContext {
    
// }

// #[derive(Debug)]
// #[non_exhaustive]
// pub struct PlayerContext<'r, Root> {
//     pub root: &'r Root,
//     pub player: player::Id,
// }

// #[derive(Debug)]
// #[derive(thiserror::Error)]
// pub enum Error<InitError> {
//     Lookup(#[from] item::lookup::canonical::Error),
//     #[error(transparent)]
//     Init(InitError),
// }