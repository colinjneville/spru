use std::borrow::Cow;

use crate::{Transaction, action, common::error::RecoverableError, item, player, record, server::error::RemovePlayerError, transaction::Transactions};

#[derive(Debug, thiserror::Error)]
#[error("Item {invalid_id} is not in the client's allowed range ({range})")]
struct NotInRangeError {
    range: item::id::Range,
    invalid_id: item::Id,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Details {
    reservation_range: item::id::Range,
    /// A client is given the reservation_range to create new item ids.
    /// As far as the server is concerned, the client can choose any id 
    /// to begin an interation, and all created ids will be sequential from
    /// there. This allows the client to easily discard ids from aborted uncommited
    /// interactions without syncing with the server.
    /// This causes a problem when reseeding, because the client no longer knows 
    /// its current Id position, and the server never knew it. Instead, the server
    /// tracks the highest committed Id, and reseeds the client to use that as the 
    /// start of its range. Even if the client had locally used greater ids, those 
    /// are now void and can be reused. 
    high_watermark: item::Id,
    status: player::Status,
}

impl Details {
    pub(crate) fn new(reservation_range: item::id::Range) -> Self {
        let high_watermark = reservation_range.start();
        Self {
            reservation_range,
            high_watermark,
            status: player::Status::Active,
        }
    }

    /// Each client is given a block of [item::Id]s to assign newly created items so they
    /// don't need to fetch one from the server every time. But a malicious client could
    /// send records with ids outside its range, leading to conflicts later. 
    /// We also reject anything from a non-active player.
    /// Return the max item Id, as it will be used to record the high watermark
    pub(crate) fn check_created_ids(
        &self,
        player_id: player::Id,
        expected_versions: &item::version::Expected,
    ) -> action::Result<Option<item::Id>> {
        if !self.status.is_active() {
            return Err(player::Error::InvalidStatus { player_id, invalid_status: self.status }.into())
        }

        let mut id_max = None;

        for &(id, version) in &expected_versions.expected {
            id_max = id_max.max(Some(id));

            // Only check item creation (i.e. before version is 0). Any client can modify any item
            // if the server OKs it, we just don't want id conflicts on created items.
            if version == item::Version::ZERO && !self.reservation_range.contains(id) {
                return Err(NotInRangeError {
                    range: self.reservation_range.clone(),
                    invalid_id: id,
                }
                .into());
            }
        }

        Ok(id_max)
    }

    pub(crate) fn status(&self) -> player::Status {
        self.status
    }

    pub(crate) fn reservation_range(&self) -> item::id::Range {
        self.reservation_range.clone()
    }

    pub(crate) fn high_watermark(&self) -> item::Id {
        self.high_watermark
    }

    pub(crate) fn make_watermark(&mut self, id: item::Id) {
        self.high_watermark = self.high_watermark.max(id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Manager<PlayerInit> {
    init: PlayerInit,
    player_details: Vec<Details>,
}

impl<PlayerInit> Manager<PlayerInit> {
    pub(crate) fn new(init: PlayerInit) -> Self {
        Self {
            init,
            player_details: vec![],
        }
    }

    pub(crate) fn add<'r>(
        &mut self,
        mut interactor: player::init::Interactor<'_, 'r, PlayerInit>,
        reservation_range: item::id::Range,
        input: PlayerInit::In,
    ) -> Result<
        player::init::Complete<'r, PlayerInit::Action, PlayerInit::Root>,
        RecoverableError<player::init::Error>,
    >
    where
        PlayerInit:
            player::Init,
    {
        let id = player::Id(self.player_details.len() as u32);
        interactor.context_mut().player = id;

        let init_error = self
            .init
            .initialize(&mut interactor, input)
            .map_err(|e| e.with_context(&self.init))
            .err();

        let complete = interactor.complete(init_error)?;
        self.player_details.push(Details::new(reservation_range));
        Ok(complete)
    }

    pub(crate) fn revert_add(&mut self) {
        self.player_details.pop().expect("No player to revert");
    }

    pub(crate) fn remove<'r>(
        &mut self, 
        mut interactor: player::init::Interactor<'_, 'r, PlayerInit>,
    ) 
        -> Result<
            player::init::Complete<'r, PlayerInit::Action, PlayerInit::Root>,
            RecoverableError<RemovePlayerError>,
        >
    where
        PlayerInit: player::Init,
    {
        let player_id = interactor.context().player;
        let details = self
            .get(player_id)
            .map_err(RemovePlayerError::Player)
            .map_err(RecoverableError::new)?;

        if details.status.is_removed() {
            Err(RecoverableError::new(RemovePlayerError::Player(player::Error::InvalidStatus { player_id, invalid_status: player::Status::Removed })))
        } else {
            let remove_error = self
                .init
                .remove(&mut interactor)
                .map_err(|e| e.with_context(&self.init))
                .err();

            let complete = interactor.complete(remove_error)
                .map_err(|e| e.map_with(RemovePlayerError::PlayerRemove))?;

            self.player_details[player_id.0 as usize].status = player::Status::Removed;
            Ok(complete)
        }
    }

    pub(crate) fn deactivate(&mut self, player_id: player::Id) -> Result<bool, player::Error> {
        let details = self.get_mut(player_id)?;
        match &mut details.status {
            player::Status::Active => {
                details.status = player::Status::Inactive;
                Ok(true)
            }
            player::Status::Inactive => Ok(false),
            player::Status::Removed => Err(player::Error::InvalidStatus { player_id, invalid_status: player::Status::Removed }),
        }
    }

    pub(crate) fn reactivate(&mut self, player_id: player::Id) -> Result<bool, player::Error> {
        let details = self.get_mut(player_id)?;
        match &mut details.status {
            player::Status::Active => Ok(false),
            player::Status::Inactive => {
                details.status = player::Status::Active;
                Ok(true)
            }
            player::Status::Removed => Err(player::Error::InvalidStatus { player_id, invalid_status: player::Status::Removed }),
        }
    }

    pub(crate) fn iter_active(&self) -> impl Iterator<Item = player::Id> {
        self.player_details
            .iter()
            .enumerate()
            .filter_map(|(i, player_details)| {
                player_details.status.is_active().then_some(player::Id(i as u32))
            })
    }

    pub(crate) fn get(&self, player_id: player::Id) -> Result<&Details, player::Error> {
        self.player_details
            .get(player_id.0 as usize)
            .ok_or(player::Error::DoesNotExist { player_id })
    }

    pub(crate) fn get_mut(&mut self, player_id: player::Id) -> Result<&mut Details, player::Error> {
        self.player_details
            .get_mut(player_id.0 as usize)
            .ok_or(player::Error::DoesNotExist { player_id })
    }

    /// Redact updates to items not visible to the indicated player.
    /// Records for invisible items will be removed, and must be later sent when the item is revealed
    /// to that player.
    /// Currently a no-op until visibility is implemented.
    pub(crate) fn redact<'tx, Action: Clone>(&self, player_id: player::Id, mut transaction: Transaction<Action>)
        -> Transaction<Action>
    {
        transaction.retain(|_packed: &record::Packed<Action>| {
            true
        });

        transaction
    }
}
