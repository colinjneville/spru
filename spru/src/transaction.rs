use std::{collections::VecDeque, fmt};

use crate::{action, error::RecoverableResult, log, record::{self, Records}, transaction};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
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

    pub(crate) fn into_iter(self) -> impl DoubleEndedIterator<Item = transaction::Confirmed<Action>> {
        self.transactions.into_iter()
            .enumerate()
            .map(move |(i, tx)| transaction::Confirmed::new(transaction::Id::new(self.start_id.get() + i), tx))
    }

    pub(crate) fn drain(&mut self) -> impl DoubleEndedIterator<Item = transaction::Confirmed<Action>> + '_ {
        let old_start_id = self.start_id.get();
        self.start_id = transaction::Id::new(old_start_id + self.transactions.len());
        self.transactions.drain(..)
            .enumerate()
            .map(move |(i, tx)| transaction::Confirmed::new(transaction::Id::new(old_start_id + i), tx))
    }

    pub fn start_id(&self) -> transaction::Id {
        self.start_id
    }

    pub fn next_id(&self) -> transaction::Id {
        transaction::Id::new(self.start_id.0 + self.transactions.len())
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

    pub(crate) fn pop_front(&mut self) -> Option<Transaction<Action>> {
        if let Some(transaction) = self.transactions.pop_front() {
            self.start_id = self.start_id.next();
            Some(transaction)
        } else {
            None
        }
    }

    pub(crate) fn pop_back(&mut self) -> Option<Transaction<Action>> {
        self.transactions.pop_back()
    }

    pub(crate) fn trim_start(&mut self, id: transaction::Id) {
        if let Some(mut diff) = id.index_of(&self.start_id) {
            diff = diff.min(self.transactions.len());
            self.transactions.drain(diff..);
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Transaction<Action> {
    records: Records<Action>,
}

impl<Action> Transaction<Action> {
    pub(crate) fn new(records: Records<Action>) -> Self {
        Self {
            records,
        }
    }

    pub(crate) fn apply<Lookup>(&self, lookup: &mut Lookup) 
        -> action::Result<Transaction<Action>> 
    where 
        Action: crate::Action<Lookup, Undo = Action>, 
    {
        let undo_records = self.records.apply(lookup)?;

        Ok(Transaction { records: undo_records })
    }

    pub(crate) fn apply_or_revert<Lookup>(&self, lookup: &mut Lookup) 
        -> RecoverableResult<Transaction<Action>, action::Error> 
    where 
        Action: crate::Action<Lookup, Undo = Action>, 
    {
        let undo_records = self.records.apply_or_revert(lookup)?;

        Ok(Transaction { records: undo_records })
    }

    pub(crate) fn records(&self) -> &Records<Action> {
        &self.records
    }

    pub(crate) fn into_records(self) -> Records<Action> {
        self.records
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Id(usize);

impl Id {
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    pub const ZERO: Self = Self(0);

    pub(crate) fn get(&self) -> usize {
        self.0
    }

    pub(crate) fn index_of(&self, start_id: &Self) -> Option<usize> {
        self.0.checked_sub(start_id.0)
    }

    pub(crate) fn next(&self) -> Self {
        Self::new(self.get() + 1)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

pub mod id {
    #[derive(Debug)]
    #[derive(thiserror::Error)]
    #[error("Transaction {0} does not exist")]
    pub struct InvalidError(pub super::Id);

    #[derive(Debug)]
    #[derive(thiserror::Error)]
    #[error("Expected transaction {expected} but received {actual}")]
    pub struct MismatchError {
        pub expected: super::Id,
        pub actual: super::Id,
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Confirmed<Action> {
    pub id: transaction::Id,
    pub transaction: Transaction<Action>,
}

impl<Action> Confirmed<Action> {
    pub(crate) fn new(id: transaction::Id, transaction: Transaction<Action>) -> Self {
        Self {
            id,
            transaction,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct Pending(pub(crate) transaction::Id);

impl Pending {
    pub(crate) const ZERO: Self = Self(transaction::Id::ZERO);

    pub(crate) fn next(&self) -> Self {
        Self(self.0.next())
    }
}
