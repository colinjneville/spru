use derive_where::derive_where;

use crate::{client, player};

#[must_use]
#[derive(Debug)]
pub struct Ret {
    
}

pub type Result<Server> = std::result::Result<super::Output<Server, Ret>, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}