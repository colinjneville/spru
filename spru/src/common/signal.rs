//! Abstracted communication between client and server.

use derive_where::derive_where;

use crate::{common, interaction, record::Records, transaction};

/// A signal sent from a [Client](crate::Client) to the [Server](crate::Server).
/// A signal is an abstraction of client-server communication and
/// must be delivered by a higher layer (such as spru-bevy).
/// Signals from the same client must be delivered in the order they are
/// generated.
#[derive_where(Debug, Serialize, Deserialize; ToServerInternal<Common>)]
pub struct ToServer<Common: crate::Common> {
    pub(crate) seq: common::SeqId,
    pub(crate) signal: ToServerInternal<Common>,
}

#[derive(derive_more::From)]
#[derive_where(Debug, Serialize, Deserialize; ApplyInteraction<Common>)]
pub(crate) enum ToServerInternal<Common: crate::Common> {
    ApplyInteraction(ApplyInteraction<Common>),
}

#[derive_where(Debug, Serialize, Deserialize; interaction::Staged<Common::Interaction>)]
pub(crate) struct ApplyInteraction<Common: crate::Common> {
    pub interaction: interaction::Staged<Common::Interaction>,
}

/// A signal sent from the [Server](crate::Server) to a [Client](crate::Client).
/// A signal is an abstraction of client-server communication and
/// must be delivered by a higher layer (such as spru-bevy).
/// Signals to the same client must be delivered in the order they are
/// generated.
#[derive_where(Debug, Serialize, Deserialize; ToClientInternal<Common>)]
pub struct ToClient<Common: crate::Common> {
    pub(crate) seq: common::SeqId,
    pub(crate) signal: ToClientInternal<Common>,
}

#[derive(derive_more::From)]
#[derive_where(Debug, Serialize, Deserialize; InteractionResult<Common>, ConfirmedTransaction<Common>, EndGame<Common>)]
pub(crate) enum ToClientInternal<Common: crate::Common> {
    InteractionResult(InteractionResult<Common>),
    ConfirmedTransaction(ConfirmedTransaction<Common>),
    EndGame(EndGame<Common>),
}

#[derive_where(Debug, Serialize, Deserialize; Records<Common::Action>)]
pub(crate) struct InteractionResult<Common: crate::Common> {
    pub pending_interaction_id: interaction::Pending,
    pub confirmed_transaction_id: Option<(transaction::Id, Records<Common::Action>)>,
}

#[derive_where(Debug, Serialize, Deserialize; transaction::Confirmed<Common::Action>)]
pub(crate) struct ConfirmedTransaction<Common: crate::Common> {
    pub confirmed_transaction: transaction::Confirmed<Common::Action>,
}

#[derive_where(Debug, Serialize, Deserialize; Common::GameOutcome)]
pub(crate) struct EndGame<Common: crate::Common> {
    pub game_outcome: Common::GameOutcome,
}
