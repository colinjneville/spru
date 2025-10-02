use derive_where::derive_where;

use crate::{interaction, transaction, SeqId};

#[derive_where(Debug, Serialize, Deserialize; Internal<Common>)]
pub struct Arg<Common: crate::Common> {
    pub(crate) seq: SeqId,
    pub(crate) signal: Internal<Common>,
}

#[derive(Debug)]
#[must_use]
pub struct Ret {

}

pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
}

#[derive(derive_more::From)]
#[derive_where(Debug, Serialize, Deserialize; ApplyInteraction<Common>)]
pub(crate) enum Internal<Common: crate::Common> {
    ApplyInteraction(ApplyInteraction<Common>),
}

#[derive_where(Debug, Serialize, Deserialize; interaction::Staged<Common::Interaction>)]
pub(crate) struct ApplyInteraction<Common: crate::Common> {
    pub interaction: interaction::Staged<Common::Interaction>,
    pub pending_transaction_id: transaction::Pending,
}

