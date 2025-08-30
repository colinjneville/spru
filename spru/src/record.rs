use std::{collections::VecDeque, fmt};

use crate::{action, item::{self, lookup}, log};


#[derive(Debug)]
pub enum Error {
    Lookup(lookup::Error),
    Item(item::id::Error),
    Version(item::version::Error),
    Action(action::Error),
}

impl From<lookup::Error> for Error {
    fn from(value: lookup::Error) -> Self {
        Self::Lookup(value)
    }
}

impl From<item::id::Error> for Error {
    fn from(value: item::id::Error) -> Self {
        Self::Item(value)
    }
}

impl From<item::version::Error> for Error {
    fn from(value: item::version::Error) -> Self {
        Self::Version(value)
    }
}

impl From<action::Error> for Error {
    fn from(value: action::Error) -> Self {
        Self::Action(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lookup(e) => fmt::Display::fmt(e, f),
            Error::Item(e) => fmt::Display::fmt(e, f),
            Error::Version(e) => fmt::Display::fmt(e, f),
            Error::Action(e) => fmt::Display::fmt(e, f),
        }
    }
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Packed<Action> {
    item_id: item::Id,
    version_change: item::version::Change,
    action: Action,
    appended_actions: Vec<Action>,
}

impl<Action> Packed<Action> {
    pub(crate) fn new(item_id: item::Id, version_change: item::version::Change, action: Action) -> Self {
        Self {
            item_id,
            version_change,
            action,
            appended_actions: vec![],
        }
    }

    pub fn append(&mut self, action: Action) {
        self.appended_actions.push(action);
    }

    pub(crate) fn item_id(&self) -> item::Id {
        self.item_id
    }

    pub(crate) fn version_change(&self) -> item::version::Change {
        self.version_change
    }

    pub(crate) fn expand(&self) -> impl Iterator<Item = Record<Action>> {
        (0..self.appended_actions.len() + 1).into_iter()
            .map(|i| Record::new(self, i))
    }

    pub(crate) fn into_action(self) -> Action {
        self.action
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Record<'r, Action> {
    packed: &'r Packed<Action>,
    index: usize,
}

impl<'r, Action> Record<'r, Action> {
    pub(crate) fn new(packed: &'r Packed<Action>, index: usize) -> Self {
        Self {
            packed,
            index,
        }
    }

    fn apply_internal<Lookup>(&self, lookup: &mut Lookup) -> self::Result<Option<Action>> 
    where 
        Action: crate::Action<Lookup, Undo = Action>,  
    {
        let undo = self.action().apply(action::Context::new(lookup, self.item_id().clone(), self.version_change()))?;
        Ok(undo)
    }

    pub(crate) fn item_id(&self) -> item::Id {
        self.packed.item_id
    }

    pub(crate) fn version_change(&self) -> item::version::Change {
        self.packed.version_change
    }

    pub(crate) fn action(&self) -> &Action {
        match self.index.checked_sub(1) {
            Some(index) => &self.packed.appended_actions[index],
            None => &self.packed.action,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Records<Action> {
    records: VecDeque<Packed<Action>>,
}

impl<Action> Records<Action> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, record: Packed<Action>) {
        self.records.push_back(record);
    }

    pub fn extend(&mut self, other: Self) {
        self.records.extend(other.records);
    }

    pub fn iter(&self) -> impl Iterator<Item = Record<Action>> {
        self.records.iter()
            .flat_map(Packed::expand)
    }

    pub(crate) fn trim_start(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.records.pop_front()
                .expect("not enough records");
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }
}

impl<Action> Default for Records<Action> {
    fn default() -> Self {
        Self { records: Default::default() }
    }
}

impl<Action> Records<Action> {
    pub fn apply<'l, Lookup>(&self, lookup: &'l mut Lookup) 
        -> self::Result<Self> 
    where 
        Action: crate::Action<Lookup, Undo = Action>, 
    {
        self.apply_internal(lookup)
            .map_err(|(_, e)| e)
    }

    pub fn apply_or_revert<'l, Lookup>(&self, lookup: &'l mut Lookup) 
        -> log::Result<Self> 
    where 
        Action: crate::Action<Lookup, Undo = Action>, 
    {
        match self.apply_internal(lookup) {
            Ok(undo) => Ok(undo),
            Err((undo, e)) => {
                if let Err(e2) = undo.apply(lookup) {
                    Err(log::Error::Revert(log::RevertError { initial: Some(e), fatal: e2 }))
                } else {
                    Err(log::Error::Record(e))
                }
            }
        }
    }

    fn apply_internal<'l, Lookup>(&self, lookup: &'l mut Lookup) 
        -> std::result::Result<Self, (Self, self::Error)> 
    where 
        Action: crate::Action<Lookup, Undo = Action> 
    {
        let mut undo = Self::default();

        for r in self.iter() {
            match r.apply_internal(lookup) {
                Ok(Some(undo_action)) => {
                    if let Some(back) = undo.records.back_mut() {
                        // Pack the undo records if the original record was packed
                        if back.item_id == r.item_id() {
                            back.append(undo_action);
                            continue;
                        }

                        // Otherwise start a new undo record
                        let packed = Packed::new(r.item_id(), r.version_change().undo(), undo_action);
                        undo.records.push_back(packed);
                    }
                }
                Ok(None) => { }
                Err(e) => return Err((undo, e)),
            }
        }

        Ok(undo)
    }
}
