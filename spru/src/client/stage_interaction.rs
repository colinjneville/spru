use crate::transaction;

#[derive(Debug)]
pub struct Arg<Interaction> {
    pub interaction: Interaction,
}

#[derive(Debug)]
#[must_use]
pub struct Ret {
    pub pending_transaction_id: transaction::Pending,
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}