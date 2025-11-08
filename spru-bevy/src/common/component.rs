use std::{fmt, ops};

use bevy::prelude;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    prelude::Component,
)]
#[component(immutable)]
pub struct GameId(spru::game::Id);

impl GameId {
    pub(crate) fn new(game_id: spru::game::Id) -> Self {
        Self(game_id)
    }
}

impl ops::Deref for GameId {
    type Target = spru::game::Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, prelude::Component)]
pub struct Root<Common: super::CommonSSS>(pub Common::Root);

impl<Common: super::CommonSSS> Root<Common> {
    pub(crate) fn new(root: Common::Root) -> Self {
        Self(root)
    }
}

impl<Common: super::CommonSSS> ops::Deref for Root<Common> {
    type Target = Common::Root;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Common: super::CommonSSS> fmt::Display for Root<Common>
where
    Common::Root: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
