pub mod error;
pub use error::Error;

use std::{borrow::Cow, collections::VecDeque, fmt};

use derive_where::derive_where;
use tagset::tagset_meta;
use telety::telety;

use crate::{interactor, item, player};

/// Additional context available during an [trait@Interaction].
#[derive(Debug)]
#[non_exhaustive]
pub struct Context<'r, Root> {
    /// The game [Root](crate::Common::Root)
    pub root: &'r Root,
    /// The id of the player who initiated the [trait@Interaction]
    pub player: player::Id,
}

impl<'r, Root> interactor::PlayerContext for Context<'r, Root> {
    fn player_context(&self) -> Option<player::Id> {
        Some(self.player)
    }
}

impl<'r, Root> crate::interactor::GetRoot for Context<'r, Root> {
    type Root = Root;

    fn get_root(&self) -> &Self::Root {
        self.root
    }
}

#[derive(Debug)]
#[derive_where(Default)]
#[doc(hidden)]
pub struct Output<Trigger> {
    pub(crate) triggers: VecDeque<Trigger>,
}

impl<Trigger> interactor::EnqueueTrigger for Output<Trigger> {
    type Trigger = Trigger;

    fn enqueue_trigger(&mut self, trigger: Trigger) {
        self.triggers.push_back(trigger);
    }
}

impl<Trigger> interactor::TakeTriggers<Trigger> for Output<Trigger> {
    fn take_triggers(&mut self) -> VecDeque<Trigger> {
        std::mem::take(&mut self.triggers)
    }
}

impl<Trigger, GameOutcome> interactor::TakeGameOutcome<GameOutcome> for Output<Trigger> {
    fn take_game_outcome(&mut self) -> Option<GameOutcome> {
        None
    }
}

impl<'r, Root> Context<'r, Root> {
    pub(crate) fn new(root: &'r Root, player: player::Id) -> Self {
        Self { root, player }
    }
}

// TODO temporary workaround for `Interaction::apply` bound. Telety isn't translating `crate::Action` to `spru::Action`
extern crate self as spru;

/// A client-initiated change to the game state.  
///
/// The Interaction handles the ways players can interact with the game. An Interaction is usually
/// analagous to a "move" in a game, often a piece of a larger turn, but conceptually a single action.
/// Some examples of potential Interactions include:  
/// * playing a card, along with paying any resource costs
/// * moving a piece, and capturing any opposing piece it lands on
/// * deciding to end your turn
///
/// # Guidelines for Interactions:
/// * Break Interactions into discrete decisions. For example drawing a card involves
///   removing the top card from the deck, then adding it to hand, but there is only one decision
///   made. Once the card is removed from the deck, there is no choice but to add it to hand.
/// * Don't interact with items unknown to the player.  
///   Interactions are run locally, so they should not access hidden information, such as the
///   top card of the deck, or a face-down tile. Instead, the Interaction should issue a
///   [Trigger](crate::Reaction::Trigger), so a server-side [Reaction](trait@crate::Reaction)
///   can handle it.
/// * Only read and write items as necessary, as this minimizes conflicting Interactions when multiple
///   players act simultaneously.
/// * Interactions should validate during [Interaction::apply]. A cheating client could send any
///   representable Interaction to the server, so make sure validation is done here, not just
///   in the client UI, etc.
///
/// # Applying Interactions
/// Interactions are a 2 step process, they are first applied locally on a client, then the client
/// can either revert the Interaction, or attempt to apply it to the server. The server will apply
/// the Interaction, then it will start a Reaction for each queued Trigger. If these are all successful,
/// the sending client will be notified it can commit its local changes, and it will be sent all the
/// changes from the Reactions. All other clients will be sent the whole set of changes. If the server-side
/// application fails, the sending client will be instructed to revert its local changes. Other clients
/// will not be notified.
///
/// # Simultaneous Interactions
/// Each client/player can stage and apply interactions simultaneously. When a client stages an Interaction,
/// it notes the version number of each item it reads or writes. It sends this information to the server when
/// it attempts to apply the Interaction. If any of the version numbers do not match the server's current
/// version (i.e. an item was written while another client read or wrote it), the server rejects the
/// Interaction. Also, if a client has a staged Interaction, and the server notifies it of confirmed changes
/// that touch any items read or written by the staged Interaction, the client will forcibly revert the Interaction.
/// For this reason, Interactions should be granular, and writes to commonly read items (such as
/// the game root) should be minimized.
#[telety(crate::interaction, alias_traits = "always")]
#[tagset_meta]
pub trait Interaction {
    /// The game's [trait@crate::Action]
    type Action: crate::Action;
    /// See [Common::Root](crate::Common::Root).
    type Root;
    /// See [Reaction::Trigger](crate::Reaction::Trigger)
    type Trigger;

    /// How the Interaction is applied.  
    ///
    /// Changes to the game state are made through the [Interactor](crate::Interactor). If this
    /// function returns an error, all changes will be reverted.
    fn apply<'l, 'r, Storage>(
        &self,
        interactor: &mut Interactor<'l, 'r, Storage, Self>,
    ) -> self::Result<()>
    where
        Storage: item::Storage<State = <Self::Action as spru::Action>::State>;

    /// A description of the Interaction, usually for diagnostic purposes
    fn description(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }
}

/// An alias for the [Interactor](crate::Interactor) used in an [Interaction](trait@Interaction).
pub type Interactor<'l, 'r, Storage, Interaction> = crate::Interactor<
    'l,
    Storage,
    <Interaction as self::Interaction>::Action,
    Context<'r, <Interaction as self::Interaction>::Root>,
    Output<<Interaction as self::Interaction>::Trigger>,
>;

/// A result with an [Error] `Err`
pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Staged<Interaction> {
    pub(crate) interaction: Interaction,
    pub(crate) expected_versions: item::version::Expected,
    pub(crate) pending_interaction_id: Pending,
}

/// An identifier for a locally staged [trait@Interaction].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[repr(transparent)]
pub struct Pending(u32);

impl Pending {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "i{}", self.0)
    }
}
