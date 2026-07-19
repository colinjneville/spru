use std::collections::VecDeque;

use crate::{action, common::error::RecoverableError, item};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Packed<Action> {
    item_id: item::Id,
    version_change: item::version::Change,
    action: Action,
    appended_actions: Vec<Action>,
}

impl<Action> Packed<Action> {
    pub(crate) fn new(
        item_id: item::Id,
        version_change: item::version::Change,
        action: Action,
    ) -> Self {
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

    pub(crate) fn expand(&self) -> impl Iterator<Item = Record<'_, Action>> {
        (0..self.appended_actions.len() + 1).map(|i| Record::new(self, i))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'r, Action> {
    packed: &'r Packed<Action>,
    index: usize,
}

impl<'r, Action> Record<'r, Action> {
    pub(crate) fn new(packed: &'r Packed<Action>, index: usize) -> Self {
        Self { packed, index }
    }

    fn apply_internal<Storage>(&self, storage: &mut Storage) -> action::Result<Option<Action>>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        let undo = self.action().apply(action::Context::new(
            storage,
            self.item_id(),
            self.version_change(),
        ))?;
        Ok(undo)
    }

    pub(crate) fn item_id(&self) -> item::Id {
        self.packed.item_id
    }

    pub(crate) fn version_change(&self) -> item::version::Change {
        // Only change the version number on the first action
        if self.index == 0 {
            self.packed.version_change
        } else {
            self.packed.version_change.into_noop()
        }
    }

    pub(crate) fn action(&self) -> &Action {
        match self.index.checked_sub(1) {
            Some(index) => &self.packed.appended_actions[index],
            None => &self.packed.action,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    pub fn iter(&self) -> impl Iterator<Item = Record<'_, Action>> {
        self.records.iter().flat_map(Packed::expand)
    }

    pub(crate) fn trim_start(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.records.pop_front().expect("not enough records");
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn retain<F>(&mut self, f: F)
    where
        F: Fn(&Packed<Action>) -> bool,
    {
        self.records.retain(f);
    }

    pub(crate) fn any<F>(&self, f: F)
        -> bool
    where
        F: Fn(&Packed<Action>) -> bool,
    {
        self.records.iter().any(f)
    }
}

impl<Action> Default for Records<Action> {
    fn default() -> Self {
        Self {
            records: Default::default(),
        }
    }
}

impl<Action> Records<Action> {
    pub fn apply<Storage>(&self, storage: &mut Storage) -> action::Result<Self>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        self.apply_internal(storage).map_err(|(_, e)| e)
    }

    pub fn apply_or_revert<Storage>(
        &self,
        storage: &mut Storage,
    ) -> Result<Self, RecoverableError<action::Error>>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        match self.apply_internal(storage) {
            Ok(undo) => Ok(undo),
            Err((undo, e)) => {
                let mut rec_e = RecoverableError::new(e);
                if let Err(e2) = undo.apply(storage) {
                    rec_e.set_recovery_error(e2);
                }
                Err(rec_e)
            }
        }
    }

    fn apply_internal<Storage>(
        &self,
        storage: &mut Storage,
    ) -> std::result::Result<Self, (Self, action::Error)>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        let _span = tracing::trace_span!("apply_actions").entered();

        let mut undo = Self::default();

        for r in self.iter() {
            match r.apply_internal(storage) {
                Ok(Some(undo_action)) => {
                    // Pack the undo records if the original record was packed
                    if let Some(back) = undo.records.back_mut() && back.item_id == r.item_id() {
                        back.append(undo_action);
                    } else {
                        // Otherwise start a new undo record
                        let packed = Packed::new(r.item_id(), r.version_change().undo(), undo_action);
                        undo.records.push_back(packed);
                    }
                }
                Ok(None) => {}
                Err(e) => return Err((undo, e)),
            }
        }

        Ok(undo)
    }
}
