use derive_where::derive_where;

use crate::{interaction, transaction, common};

#[derive_where(Debug, Serialize, Deserialize; Internal<Common>)]
pub struct Signal<Common: crate::Common> {
    pub(crate) seq: common::SeqId,
    pub(crate) signal: Internal<Common>,
}

#[derive(Debug)]
#[must_use]
pub struct Ret {

}

pub type Result<Server> = std::result::Result<super::Output<Server, Ret>, self::Error>;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error(" An unrecoverable error occurred: {0}")]
    Fatal(crate::action::Error),
}

#[derive(derive_more::From)]
#[derive_where(Debug, Serialize, Deserialize; ApplyInteraction<Common>)]
pub(crate) enum Internal<Common: crate::Common> {
    ApplyInteraction(ApplyInteraction<Common>),
}

#[derive_where(Debug, Serialize, Deserialize; interaction::Staged<Common::Interaction>)]
pub(crate) struct ApplyInteraction<Common: crate::Common> {
    pub interaction: interaction::Staged<Common::Interaction>,
}

