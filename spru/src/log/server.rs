use std::{sync::{atomic::{self, AtomicUsize}, Arc, Mutex}};

use crate::{action, error::RecoverableResult, log::error::UndoError, transaction::{self, Transactions}, Transaction};

#[derive(Debug)]
pub(crate) struct Server<Action> {
    next_id: transaction::Id,
    // Compensation transactions to be applied to undo the effects of a transaction.
    // Can be cleared when game logic decides undos are no longer allowed
    undo_transactions: Transactions<Action>,
    undo_pin_board: Arc<UndoPinBoard>,
}

impl<Action> Server<Action> {
    pub(crate) fn new() -> Self {
        Self::new_with_next_id(transaction::Id::ZERO)
        
    }

    pub(crate) fn new_with_next_id(next_id: transaction::Id) -> Self {
        Self {
            next_id,
            undo_transactions: Default::default(),
            undo_pin_board: Arc::new(UndoPinBoard::new()),
        }
    }

    pub(crate) fn next_id(&self) -> transaction::Id {
        self.next_id
    }
}

impl<Action> Server<Action> {
    fn apply_or_revert<Lookup>(&mut self, lookup: &mut Lookup, transaction: Transaction<Action>) 
        -> RecoverableResult<transaction::Confirmed<Action>, action::Error>
    where 
        Action: crate::Action<Lookup, Undo = Action>,
    {
        self.release_undo();

        let undo_transaction = transaction.apply_or_revert(lookup)?;
        
        let do_id = self.next_id;
        self.next_id = do_id.next();
        
        let undo_id = self.undo_transactions.push_back(undo_transaction);

        debug_assert!(do_id == undo_id);
        Ok(transaction::Confirmed { id: do_id, transaction })
    }

    pub(crate) fn register_undo(&mut self, undo_transaction: Transaction<Action>) -> transaction::Id {
        let do_id = self.next_id;
        self.next_id = do_id.next();
        let undo_id = self.undo_transactions.push_back(undo_transaction);

        debug_assert!(do_id == undo_id);
        undo_id
    }

    pub fn undo<Lookup>(&mut self, lookup: &mut Lookup, transaction_id: transaction::Id) 
        -> RecoverableResult<transaction::Confirmed<Action>, UndoError>
    where 
        Action: crate::Action<Lookup, Undo = Action> + Clone,
    {
        let transaction = self.undo_transactions.get(transaction_id)
            .ok_or(UndoError::Invalid(transaction::id::InvalidError(transaction_id)))?
            .clone();
        
        let confirmed = self.apply_or_revert(lookup, transaction)
            .map_err(|e| e.map_with(Into::into))?;
        
        Ok(confirmed)
    }

    pub fn pin_undo(&self) -> UndoPin {
        self.undo_pin_board.add_pin()
    }

    // Release any undo transactions that are no longer needed
    fn release_undo(&mut self) {
        let min_pin = self.undo_pin_board.min_pin().unwrap_or(self.next_id);
        self.undo_transactions.trim_start(min_pin);
    }
}

#[derive(Debug, Default)]
struct UndoPinBoard {
    // All pins sorted in descending order
    sorted_pins: Mutex<Vec<transaction::Id>>,
    min_pin: AtomicUsize,
}

impl UndoPinBoard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pin(self: &Arc<Self>) -> UndoPin {
        UndoPin::new(self.clone())
    }

    pub fn min_pin(&self) -> Option<transaction::Id> {
        let raw = self.min_pin.load(atomic::Ordering::Acquire);
        if raw == 0 {
            None
        } else {
            Some(transaction::Id::new(raw - 1))
        }
    }
}

#[derive(Debug)]
pub struct UndoPin {
    current_pin: transaction::Id,
    pin_board: Arc<UndoPinBoard>,
}

impl UndoPin {
    fn new(pin_board: Arc<UndoPinBoard>) -> Self {
        
        let current_pin = if let Ok(mut sorted_pins) = pin_board.sorted_pins.try_lock() {
            let current_pin = match pin_board.min_pin() {
                Some(p) => p,
                None => {
                    let p = transaction::Id::ZERO;
                    pin_board.min_pin.store(p.get() + 1, atomic::Ordering::Release);
                    p
                },
            };
            
            sorted_pins.push(current_pin);
            current_pin
        } else {
            transaction::Id::ZERO
        };

        Self {
            current_pin,
            pin_board,
        }
    }

    pub fn release_before(&mut self, id: transaction::Id) {
        if id > self.current_pin {
            if let Ok(mut sorted_pins) = self.pin_board.sorted_pins.try_lock() {
                let old_index = sorted_pins.binary_search_by_key(&std::cmp::Reverse(&self.current_pin), std::cmp::Reverse).expect("Pin not found on PinBoard");
                let (Ok(new_index) | Err(new_index)) = sorted_pins.binary_search_by_key(&std::cmp::Reverse(&id), std::cmp::Reverse);

                // Move the old pin to the new index (shifting everything between right 1)
                sorted_pins[new_index..=old_index].rotate_right(1);
                // Update the pin to the new value
                sorted_pins[new_index] = id;

                let min_id = sorted_pins.last().copied().unwrap();
                self.pin_board.min_pin.store(min_id.get() + 1, atomic::Ordering::Release);
            }

            self.current_pin = id;
        }
    }
}

impl Clone for UndoPin {
    fn clone(&self) -> Self {
        let mut undo_pin = Self::new(self.pin_board.clone());
        undo_pin.release_before(self.current_pin.clone());
        undo_pin
    }
}

impl Drop for UndoPin {
    fn drop(&mut self) {
        if let Ok(mut sorted_pins) = self.pin_board.sorted_pins.try_lock() {
            let index = sorted_pins.binary_search_by_key(&std::cmp::Reverse(&self.current_pin), std::cmp::Reverse).expect("Pin not found on PinBoard");
            sorted_pins.remove(index);

            let last_id = sorted_pins.last().copied().map(|id| id.get() + 1).unwrap_or(0);
            self.pin_board.min_pin.store(last_id, atomic::Ordering::Release);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pinning() {
        let zero = transaction::Id::ZERO;
        let two = transaction::Id::new(2);
        let five = transaction::Id::new(5);
        let board = Arc::new(UndoPinBoard::new());
        assert_eq!(board.min_pin(), None);

        let mut pin0 = board.add_pin();
        assert_eq!(board.min_pin(), Some(zero));

        pin0.release_before(two);
        assert_eq!(board.min_pin(), Some(two));

        let pin1 = pin0.clone();
        assert_eq!(board.min_pin(), Some(two));

        let mut pin2 = board.add_pin();
        pin2.release_before(five);
        assert_eq!(board.min_pin(), Some(two));

        std::mem::drop(pin0);
        assert_eq!(board.min_pin(), Some(two));

        std::mem::drop(pin1);
        assert_eq!(board.min_pin(), Some(five));

        std::mem::drop(pin2);
        assert_eq!(board.min_pin(), None);
    }
}