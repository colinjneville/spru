use std::collections::VecDeque;

use crate::{action, interaction, item, log, record, transaction, Transaction};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct PendingTransaction<ActionCatalog, Interaction> {
    id: transaction::Pending,
    undo_transaction: Transaction<ActionCatalog>,
    // Once we have sent an apply message to the server, we shouldn't allow the client
    // to attempt to revert that transaction (otherwise the revert may be forcibly
    // un-reverted if the server commits it).
    // Take the interaction when we apply to the server so we only apply once,
    // and check that we still own the interaction when reverting locally
    interaction: Option<interaction::Staged<Interaction>>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Client<ActionCatalog, Interaction> {
    // Transactions applied locally which have not been confirmed by the server.
    // Any rejected transaction rolls back all pending, because the player may
    // have made a later Interaction on the assumption the earlier one was valid.
    // TODO add some sort of configuration?
    pending_undo_transactions: VecDeque<PendingTransaction<ActionCatalog, Interaction>>,
    
    // Next pending id to use. If we cancel a staged transaction, we don't want to reuse
    // the pending id to prevent cancelling the newer staged transaction with the id
    // by mistake.
    next_pending_id: transaction::Pending,
    // Next transaction id expected from the server. If everything is implemented
    // correctly, this is not strictly necessary, but if helps us detect if we
    // have missed any confirmed transactions because of an incorrect implementation.
    next_confirmed_id: transaction::Id,
}

impl<ActionCatalog, Interaction> Client<ActionCatalog, Interaction> {
    pub(crate) fn new(next_confirmed_id: transaction::Id) -> Self {
        Self {
            pending_undo_transactions: VecDeque::new(),
            next_pending_id: transaction::Pending::ZERO,
            next_confirmed_id,
        }
    }
    pub(crate) fn apply_confirmed<Lookup>(&mut self, lookup: &mut Lookup, transaction: transaction::Confirmed<ActionCatalog>)
        -> Result<(), log::ConfirmError<Lookup::Error, ActionCatalog::Error>>
    where
        Lookup: item::Lookup,
        ActionCatalog: action::Catalog<Lookup>,
    {
        if self.next_confirmed_id == transaction.id {
            self.next_confirmed_id = self.next_confirmed_id.next();

            match transaction.transaction.apply_or_revert(lookup) {
                Ok(_) => Ok(()),
                // Failed to apply transaction, and revert 
                Err(e @ log::Error::Revert(_)) => Err(log::ConfirmError::Log(e)),
                Err(log::Error::Record(e)) => {
                    // Log apply failed, discard our local pending changes...
                    // TODO we could instead look for the earliest incompatible transaction
                    // and only rollback until there
                    match self.revert_pending(lookup, None) {
                        Ok(()) => {
                            // Then try one more time on what should be a clean slate
                            transaction.transaction.apply(lookup)
                                .map(|_| ())
                                .map_err(log::Error::Record)
                                .map_err(log::ConfirmError::Log)
                        },
                        Err(e2) => Err(log::ConfirmError::Log(log::Error::Revert(log::RevertError { initial: Some(e), fatal: e2 }))),
                    }
                },
            }
        } else {
            Err(log::ConfirmError::Mismatch(transaction::id::MismatchError { expected: self.next_confirmed_id, actual: transaction.id }))
        }
    }

    pub fn stage_pending<Lookup>(&mut self, lookup: &mut Lookup, interaction: interaction::Staged<Interaction>, transaction: Transaction<ActionCatalog>)
        -> Result<transaction::Pending, log::Error<Lookup::Error, ActionCatalog::Error>>
    where
        Lookup: item::Lookup,
        ActionCatalog: action::Catalog<Lookup>,
    {
        let id = self.next_pending_id;
        self.next_pending_id = self.next_pending_id.next();
        let undo_transaction = transaction.apply_or_revert(lookup)?;
        let pending = PendingTransaction {
            id,
            undo_transaction,
            interaction: Some(interaction),
        };
        self.pending_undo_transactions.push_back(pending);
        Ok(id)
    }

    pub fn apply_pending(&mut self, pending_transaction_id: transaction::Pending)
        -> Result<Vec<interaction::Staged<Interaction>>, crate::TempError>
    {
        match self.pending_undo_transactions.binary_search_by_key(&pending_transaction_id, |p| p.id) {
            Ok(index) => {
                let mut output = vec![];
                for i in 0..=index {
                    let pending = self.pending_undo_transactions.get_mut(i)
                        .expect("index obtained by search");
                    if let Some(interaction) = pending.interaction.take() {
                        output.push(interaction);
                    }
                }
                Ok(output)
            },
            // transaction not found (invalid, reverted, or commited)
            Err(_) => Err(crate::TempError),
        }
    }

    pub fn confirm_pending(&mut self, pending_transaction_id: transaction::Pending, confirmed_transaction_id: transaction::Id)
        -> Result<(), crate::TempError>
    {
        match self.pending_undo_transactions.get(0) {
            Some(pending) => {
                if pending.id == pending_transaction_id {
                    if self.next_confirmed_id == confirmed_transaction_id {
                        self.next_confirmed_id = self.next_confirmed_id.next();
                        self.pending_undo_transactions.pop_front();
                        Ok(())
                    } else {
                        // We expected a different confirmed transaction id
                        Err(crate::TempError)
                    }
                } else {
                    // We expected a different pending tranaction id
                    Err(crate::TempError)
                }
            }
            // No pending transactions
            None => Err(crate::TempError),
        }
    }

    pub fn revert_pending<Lookup>(&mut self, lookup: &mut Lookup, until: Option<transaction::Pending>) 
        -> Result<(), record::Error<Lookup::Error, ActionCatalog::Error>> 
    where 
        Lookup: item::Lookup,
        ActionCatalog: action::Catalog<Lookup>,
    {       
        while let Some(pending) = self.pending_undo_transactions.pop_back() {
            if Some(pending.id) < until {
                // We've gone too far, put it back and exit
                self.pending_undo_transactions.push_back(pending);
                break;
            } else {
                if pending.interaction.is_some() {
                    pending.undo_transaction.apply(lookup)?;
                } else {
                    // TODO CORRECTNESS
                    // If this revert is due to a conflict with an incoming server transaction, we need
                    // to be able to revert past the lock point because those pending changes may be the
                    // cause of the conflict. 
                    todo!()
                }
            }
        }

        Ok(())
    }
}