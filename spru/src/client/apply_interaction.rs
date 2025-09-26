use crate::transaction;

#[derive(Debug)]
pub struct Arg {
    pub pending_transaction_id: transaction::Pending,
}

#[derive(Debug)]
pub struct Ret {

}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}