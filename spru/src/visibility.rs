#![allow(dead_code)]
#![allow(missing_docs)]

use std::{
    collections::{HashMap, hash_map},
    ops,
};

use crate::{item, player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Visible,
    Hidden(item::Version),
}

#[derive(Debug)]
pub struct Manager {
    partially_visible: HashMap<item::Id, PlayerStatus>,
}

impl Manager {
    pub(crate) fn new() -> Self {
        Self {
            partially_visible: HashMap::default(),
        }
    }
}

impl Manager {
    pub fn show(&mut self, id: &item::Id, specific_player: Option<player::Id>) {
        if let hash_map::Entry::Occupied(mut oe) = self.partially_visible.entry(*id) {
            if let Some(specific_player) = specific_player {
                if oe
                    .get_mut()
                    .set_visibility(specific_player, Visibility::Visible)
                {
                    oe.remove();
                }
            } else {
                oe.remove();
            }
        }
    }

    // pub fn hide(&mut self, id: &Id, version: Version, specific_player: Option<player::Id>) {
    //     self.partially_visible.entry(id.clone()).
    // }
}

impl ops::Index<item::Id> for Manager {
    type Output = PlayerStatus;

    fn index(&self, index: item::Id) -> &Self::Output {
        self.partially_visible
            .get(&index)
            .unwrap_or(&DEFAULT_STATUS)
    }
}

#[derive(Debug)]
pub struct PlayerStatus {
    status: Vec<Visibility>,
    hidden_count: usize,
}

impl PlayerStatus {
    /// Returns true if this object is now visible to all players
    fn set_visibility(&mut self, player: player::Id, visibility: Visibility) -> bool {
        let player = player.into_u32();
        let oob = self.status.len() < player as usize;
        if visibility == Visibility::Visible && oob {
            false
        } else {
            if oob {
                self.status.resize(player as usize, Visibility::Visible);
            }
            let prev_visibility = std::mem::replace(&mut self.status[player as usize], visibility);
            let diff = match (prev_visibility, visibility) {
                (Visibility::Hidden(_), Visibility::Visible) => -1,
                (Visibility::Visible, Visibility::Hidden(_)) => 1,
                _ => 0,
            };
            self.hidden_count = self.hidden_count.wrapping_add_signed(diff);
            self.hidden_count == 0
        }
    }
}

impl ops::Index<player::Id> for PlayerStatus {
    type Output = Visibility;

    fn index(&self, index: player::Id) -> &Self::Output {
        self.status
            .get(index.into_u32() as usize)
            .unwrap_or(&DEFAULT_VISIBILITY)
    }
}

static DEFAULT_VISIBILITY: Visibility = Visibility::Visible;
static DEFAULT_STATUS: PlayerStatus = PlayerStatus {
    status: Vec::new(),
    hidden_count: 0,
};
