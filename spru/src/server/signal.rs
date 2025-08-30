use crate::{interaction, transaction};



#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Arg<Interaction> {
    pub(crate) signal: Internal<Interaction>,
}

#[derive(Debug)]
#[must_use]
pub struct Ret {

}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Temp(#[from] crate::TempError),
    #[error("The server has entered an inconsistent state due to a bug: {0}")]
    Inconsistency(#[from] crate::log::RevertError),
}

#[derive(Debug)]
#[derive(derive_more::From)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum Internal<Interaction> {
    ApplyInteraction(ApplyInteraction<Interaction>),
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ApplyInteraction<Interaction> {
    pub interaction: interaction::Staged<Interaction>,
    pub pending_transaction_id: transaction::Pending,
}

