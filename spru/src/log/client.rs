use std::collections::VecDeque;

use crate::{action, error::RecoverableResult, interaction, item, log::error::ConfirmError, record::Records, transaction, Transaction};

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct PendingTransaction<Action, Interaction> {
    id: transaction::Pending,
    undo_transaction: Transaction<Action>,
    // Once we have sent an apply message to the server, we shouldn't allow the client
    // to attempt to revert that transaction (otherwise the revert may be forcibly
    // un-reverted if the server commits it).
    // Take the interaction when we apply to the server so we only apply once,
    // and check that we still own the interaction when reverting locally
    interaction: Option<interaction::Staged<Interaction>>,
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Client<Action, Interaction> {
    // Transactions applied locally which have not been confirmed by the server.
    // Any rejected transaction rolls back all pending, because the player may
    // have made a later Interaction on the assumption the earlier one was valid.
    // TODO add some sort of configuration?
    pending_undo_transactions: VecDeque<PendingTransaction<Action, Interaction>>,
    
    // Next pending id to use. If we cancel a staged transaction, we don't want to reuse
    // the pending id to prevent cancelling the newer staged transaction with the id
    // by mistake.
    next_pending_id: transaction::Pending,
    // Next transaction id expected from the server. If everything is implemented
    // correctly, this is not strictly necessary, but if helps us detect if we
    // have missed any confirmed transactions because of an incorrect implementation.
    next_confirmed_id: transaction::Id,
}

impl<Action, Interaction> Client<Action, Interaction> {
    pub(crate) fn new(next_confirmed_id: transaction::Id) -> Self {
        Self {
            pending_undo_transactions: VecDeque::new(),
            next_pending_id: transaction::Pending::ZERO,
            next_confirmed_id,
        }
    }
    pub(crate) fn apply_confirmed<Lookup>(&mut self, lookup: &mut Lookup, transaction: transaction::Confirmed<Action>)
        -> RecoverableResult<(), ConfirmError>
    where
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        if self.next_confirmed_id == transaction.id {
            self.next_confirmed_id = self.next_confirmed_id.next();

            self.apply_records(lookup, transaction.transaction.records())
                .map_err(|e| e.map_with(Into::into))
        } else {
            Err(ConfirmError::Mismatch(transaction::id::MismatchError { expected: self.next_confirmed_id, actual: transaction.id }).into())
        }
    }

    fn apply_records<Lookup>(&mut self, lookup: &mut Lookup, records: &Records<Action>)
        -> RecoverableResult<(), action::Error>
    where
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        match records.apply_or_revert(lookup) {
            Ok(_) => Ok(()),
            Err(mut re) => 
                if re.is_recovered() {
                    // Log apply failed, discard our local pending changes...
                    // TODO we could instead look for the earliest incompatible transaction
                    // and only rollback until there
                    match self.revert_pending(lookup, None) {
                        Ok(()) => {
                            // Then try one more time on what should be a clean slate
                            records.apply(lookup)
                                .map(|_| ())
                                .map_err(Into::into)
                        },
                        Err(e) => {
                            re.recovery_error = Some(e);
                            Err(re)
                        }
                    }
                } else {
                    // Failed to apply transaction, and revert
                    Err(re)
                }
        }
    }

    pub fn stage_pending(
        &mut self, 
        interaction: interaction::Staged<Interaction>, 
        undo_transaction: Transaction<Action>,
    )
        -> transaction::Pending
    {
        let id = self.next_pending_id;
        self.next_pending_id = self.next_pending_id.next();
        
        let pending = PendingTransaction {
            id,
            undo_transaction,
            interaction: Some(interaction),
        };
        self.pending_undo_transactions.push_back(pending);
        id
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
            Err(_) => Err(crate::TempError::new()),
        }
    }

    pub fn confirm_pending<Lookup>(
        &mut self, 
        lookup: &mut Lookup,
        pending_transaction_id: transaction::Pending, 
        confirmed_transaction_id: transaction::Id, 
        reaction_records: &Records<Action>
    )
        -> Result<(), crate::TempError>
    where
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        match self.pending_undo_transactions.get(0) {
            Some(pending) => {
                if pending.id == pending_transaction_id {
                    if self.next_confirmed_id == confirmed_transaction_id {
                        self.apply_records(lookup, &reaction_records)
                            .map_err(crate::TempError::discard)?;
                        
                        self.next_confirmed_id = self.next_confirmed_id.next();
                        self.pending_undo_transactions.pop_front();
                        Ok(())
                    } else {
                        // We expected a different confirmed transaction id
                        Err(crate::TempError::new())
                    }
                } else {
                    // We expected a different pending tranaction id
                    Err(crate::TempError::new())
                }
            }
            // No pending transactions
            None => Err(crate::TempError::new()),
        }
    }

    pub fn revert_pending<Lookup>(&mut self, lookup: &mut Lookup, until: Option<transaction::Pending>) 
        -> Result<(), action::Error> 
    where 
        Lookup: item::Lookup,
        Action: crate::Action<State = Lookup::State>,
    {
        // pop_back_if: https://github.com/rust-lang/rust/issues/135889
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