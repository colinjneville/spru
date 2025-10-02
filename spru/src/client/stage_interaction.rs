use derive_where::derive_where;

use crate::transaction;

#[derive_where(Debug; Client::Interaction)]
pub struct Arg<Client: super::Client> {
    pub interaction: Client::Interaction,
}

#[derive(Debug)]
#[must_use]
pub struct Ret {
    pub pending_transaction_id: transaction::Pending,
}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}