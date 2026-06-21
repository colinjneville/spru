use std::collections::VecDeque;

use tracing::instrument;

use crate::{
    Transaction, action, common::error::RecoverableError, interaction, item, record::Records,
    transaction,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PendingTransaction<Action, Interaction> {
    id: interaction::Pending,
    undo_transaction: Transaction<Action>,
    // Once we have sent an apply message to the server, we shouldn't allow the client
    // to attempt to revert that transaction (otherwise the revert may be forcibly
    // un-reverted if the server commits it).
    // Take the interaction when we apply to the server so we only apply once,
    // and check that we still own the interaction when reverting locally
    interaction: Option<interaction::Staged<Interaction>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Log<Action, Interaction> {
    // Transactions applied locally which have not been confirmed by the server.
    // Any rejected transaction rolls back all pending, because the player may
    // have made a later Interaction on the assumption the earlier one was valid.
    // TODO add some sort of configuration?
    pending_undo_transactions: VecDeque<PendingTransaction<Action, Interaction>>,

    // Next pending id to use. If we cancel a staged transaction, we don't want to reuse
    // the pending id to prevent cancelling the newer staged transaction with the id
    // by mistake.
    next_pending_id: interaction::Pending,
    // Next transaction id expected from the server. If everything is implemented
    // correctly, this is not strictly necessary, but if helps us detect if we
    // have missed any confirmed transactions because of an incorrect implementation.
    next_confirmed_id: transaction::Id,
}

impl<Action, Interaction> Log<Action, Interaction> {
    pub(crate) fn new(next_confirmed_id: transaction::Id) -> Self {
        Self {
            pending_undo_transactions: VecDeque::new(),
            next_pending_id: interaction::Pending::ZERO,
            next_confirmed_id,
        }
    }

    #[instrument(err, skip_all, fields(transaction_id = transaction.id.get()))]
    pub(crate) fn apply_confirmed<Storage>(
        &mut self,
        storage: &mut Storage,
        transaction: transaction::Confirmed<Action>,
    ) -> Result<(), RecoverableError<super::error::TransactionConfirmationError>>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        if self.next_confirmed_id == transaction.id {
            tracing::event!(name: "external_transaction", tracing::Level::DEBUG, transaction_len = transaction.transaction.records().len());

            self.next_confirmed_id = self.next_confirmed_id.next();

            self.apply_server_records(storage, transaction.transaction.records())
                .map_err(|e| e.map_with(super::error::TransactionConfirmationError::Action))
        } else {
            tracing::event!(name: "external_transaction_invalid", tracing::Level::ERROR, expected_transaction_id = self.next_confirmed_id.get());

            Err(super::error::TransactionConfirmationError::Mismatch(
                transaction::id::MismatchError {
                    expected: self.next_confirmed_id,
                    actual: transaction.id,
                },
            )
            .into())
        }
    }

    fn apply_server_records<Storage>(
        &mut self,
        storage: &mut Storage,
        records: &Records<Action>,
    ) -> Result<(), RecoverableError<action::Error>>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        match records.apply_or_revert(storage) {
            Ok(_) => Ok(()),
            Err(mut re) => {
                if re.is_recovered() {
                    // Log apply failed, discard our local pending changes...
                    // TODO we could instead look for the earliest incompatible transaction
                    // and only rollback until there
                    match self.revert_pending(storage, None, true) {
                        Ok(_count) => {
                            // Then try one more time on what should be a clean slate
                            records.apply(storage).map(|_| ()).map_err(Into::into)
                        }
                        Err(e) => {
                            re.set_recovery_error(e);
                            Err(re)
                        }
                    }
                } else {
                    // Failed to apply transaction, and revert
                    Err(re)
                }
            }
        }
    }

    pub fn stage_pending(
        &mut self,
        interaction: Interaction,
        expected_versions: item::version::Expected,
        undo_transaction: Transaction<Action>,
    ) -> interaction::Pending {
        let pending_interaction_id = self.next_pending_id;
        self.next_pending_id = self.next_pending_id.next();

        let staged = interaction::Staged {
            interaction,
            expected_versions,
            pending_interaction_id,
        };

        let pending = PendingTransaction {
            id: pending_interaction_id,
            undo_transaction,
            interaction: Some(staged),
        };
        self.pending_undo_transactions.push_back(pending);

        pending_interaction_id
    }

    pub fn apply_pending(
        &mut self,
        pending_interaction_id: Option<interaction::Pending>,
    ) -> Result<Vec<interaction::Staged<Interaction>>, super::error::InvalidInteractionPendingError>
    {
        let index = if let Some(pending_interaction_id) = pending_interaction_id {
            self.pending_undo_transactions
                .binary_search_by_key(&pending_interaction_id, |p| p.id)
        } else if !self.pending_undo_transactions.is_empty() {
            Ok(self.pending_undo_transactions.len() - 1)
        } else {
            // No pending transactions
            return Ok(vec![]);
        };

        match index {
            Ok(index) => {
                let mut output = vec![];
                for i in 0..=index {
                    let pending = self
                        .pending_undo_transactions
                        .get_mut(i)
                        .expect("index obtained by search");
                    if let Some(interaction) = pending.interaction.take() {
                        output.push(interaction);
                    }
                }
                Ok(output)
            }
            // transaction not found (invalid, reverted, or commited)
            Err(_) => Err(super::error::InvalidInteractionPendingError::new(
                pending_interaction_id.unwrap(),
            )),
        }
    }

    pub fn confirm_pending<Storage>(
        &mut self,
        storage: &mut Storage,
        pending_interaction_id: interaction::Pending,
        confirmed_transaction_id: transaction::Id,
        reaction_records: &Records<Action>,
    ) -> Result<(), super::error::ConfirmPendingError>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        match self.pending_undo_transactions.front() {
            Some(pending) => {
                if pending.id == pending_interaction_id {
                    if self.next_confirmed_id == confirmed_transaction_id {
                        self.apply_server_records(storage, reaction_records)?;

                        self.next_confirmed_id = self.next_confirmed_id.next();
                        self.pending_undo_transactions.pop_front();
                        Ok(())
                    } else {
                        // We expected a different confirmed transaction id
                        Err(super::error::TransactionOutOfOrderError::WrongConfirmdId {
                            expected: self.next_confirmed_id,
                            actual: confirmed_transaction_id,
                        }
                        .into())
                    }
                } else {
                    // We expected a different pending tranaction id
                    Err(super::error::TransactionOutOfOrderError::WrongPendingId {
                        expected: Some(pending.id),
                        actual: pending_interaction_id,
                    }
                    .into())
                }
            }
            // No pending transactions
            None => Err(super::error::TransactionOutOfOrderError::WrongPendingId {
                expected: None,
                actual: pending_interaction_id,
            }
            .into()),
        }
    }

    pub(crate) fn revert_pending<Storage>(
        &mut self,
        storage: &mut Storage,
        until: Option<interaction::Pending>,
        from_server: bool,
    ) -> Result<usize, action::Error>
    where
        Storage: item::Storage,
        Action: crate::Action<State = Storage::State>,
    {
        let mut count = 0;

        while let Some(pending) = self.pending_undo_transactions.pop_back_if(|pending| Some(pending.id) >= until) {
            // If we have not attempted to apply this interaction yet, or the server
            // has explicitly rejected it, we can revert
            if pending.interaction.is_some() || from_server {
                count += 1;
                pending.undo_transaction.apply(storage)?;
            } else {
                // We've already told the server to confirm the interaction, but have not yet
                // recieved a response. It's no longer our choice to revert.
                break;

                // TODO CORRECTNESS
                // If this revert is due to a conflict with an incoming server transaction, we need
                // to be able to revert past the lock point because those pending changes may be the
                // cause of the conflict.
                // But this must be done one interaction at a time, because we must not undo an
                // interaction the server could accept
                #[allow(unreachable_code)]
                {
                    todo!()
                }
            }
        }

        Ok(count)
    }

    pub(crate) fn pending_interactions(&self) -> impl Iterator<Item = interaction::Pending> {
        self.pending_undo_transactions
            .iter()
            // Only report unapplied pending
            .filter(|i| i.interaction.is_some())
            .map(|i| i.id)
    }
}
