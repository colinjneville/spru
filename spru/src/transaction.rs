use std::{collections::VecDeque, fmt};

use crate::{action, common::error::RecoverableError, item, record::Records, transaction};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Transactions<Action> {
    transactions: VecDeque<Transaction<Action>>,
    start_id: transaction::Id,
}

impl<Action> Default for Transactions<Action> {
    fn default() -> Self {
        Self::new(transaction::Id::new(0))
    }
}

impl<Action> Transactions<Action> {
    pub(crate) fn new(start_id: transaction::Id) -> Self {
        Self {
            transactions: Default::default(),
            start_id,
        }
    }

    pub(crate) fn into_iter(
        self,
    ) -> impl DoubleEndedIterator<Item = transaction::Confirmed<Action>> {
        self.transactions
            .into_iter()
            .enumerate()
            .map(move |(i, tx)| {
                transaction::Confirmed::new(
                    transaction::Id::new(self.start_id.get() + i as u32),
                    tx,
                )
            })
    }

    pub fn next_id(&self) -> transaction::Id {
        transaction::Id::new(self.start_id.0 + self.transactions.len() as u32)
    }

    pub(crate) fn get(&self, id: transaction::Id) -> Option<&Transaction<Action>> {
        if let Some(index) = id.index_of(&self.start_id) {
            self.transactions.get(index)
        } else {
            None
        }
    }

    pub(crate) fn push_back(&mut self, transaction: Transaction<Action>) -> transaction::Id {
        let id = self.next_id();
        self.transactions.push_back(transaction);
        id
    }

    pub(crate) fn trim_start(&mut self, id: transaction::Id) {
        if let Some(mut diff) = id.index_of(&self.start_id) {
            diff = diff.min(self.transactions.len());
            self.transactions.drain(diff..);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Transaction<Action> {
    records: Records<Action>,
}

impl<Action> Transaction<Action> {
    pub(crate) fn new(records: Records<Action>) -> Self {
        Self { records }
    }

    pub(crate) fn apply<Lookup>(&self, lookup: &mut Lookup) -> action::Result<Transaction<Action>>
    where
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        let undo_records = self.records.apply(lookup)?;

        Ok(Transaction {
            records: undo_records,
        })
    }

    pub(crate) fn apply_or_revert<Lookup>(
        &self,
        lookup: &mut Lookup,
    ) -> Result<Transaction<Action>, RecoverableError<action::Error>>
    where
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        let undo_records = self.records.apply_or_revert(lookup)?;

        Ok(Transaction {
            records: undo_records,
        })
    }

    pub(crate) fn records(&self) -> &Records<Action> {
        &self.records
    }

    pub(crate) fn into_records(self) -> Records<Action> {
        self.records
    }
}

/// Uniquely identifies a confirmed [Transaction]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub(crate) struct Id(u32);

impl Id {
    pub(crate) fn new(index: u32) -> Self {
        Self(index)
    }

    pub(crate) const ZERO: Self = Self(0);

    pub(crate) fn get(&self) -> u32 {
        self.0
    }

    pub(crate) fn index_of(&self, start_id: &Self) -> Option<usize> {
        self.0.checked_sub(start_id.0).map(|i| i as usize)
    }

    pub(crate) fn next(&self) -> Self {
        Self::new(self.get() + 1)
    }

    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

pub(crate) mod id {
    #[derive(Debug, thiserror::Error)]
    #[error("Transaction {0} does not exist")]
    pub(crate) struct InvalidError(pub(crate) super::Id);

    #[derive(Debug, thiserror::Error)]
    #[error("Expected transaction {expected} but received {actual}")]
    pub(crate) struct MismatchError {
        pub(crate) expected: super::Id,
        pub(crate) actual: super::Id,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Confirmed<Action> {
    pub id: transaction::Id,
    pub transaction: Transaction<Action>,
}

impl<Action> Confirmed<Action> {
    pub(crate) fn new(id: transaction::Id, transaction: Transaction<Action>) -> Self {
        Self { id, transaction }
    }
}
