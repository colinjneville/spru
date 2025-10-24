
use crate::{action, common::error::RecoverableError, item, player};

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Item {invalid_id} is not in the client's allowed range ({range})")]
struct NotInRangeError {
    range: item::id::Range,
    invalid_id: item::Id,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Details {
    reservation_range: item::id::Range,
}

impl Details {
    pub(crate) fn check_created_ids(&self, expected_versions: &item::version::Expected) -> action::Result<()> {
        for &(id, version) in &expected_versions.expected {
            // Only check item creation (i.e. before version is 0). Any client can modify any item
            // if the sever OKs it, we just don't want id conflicts on created items.
            if version == item::Version::ZERO {
                if !self.reservation_range.contains(&id) {
                    return Err(NotInRangeError {
                        range: self.reservation_range.clone(),
                        invalid_id: id,
                    }.into());
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
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

    pub(crate) fn add<'r, State, Action, Root> (
        &mut self, 
        mut interactor: player::init::Interactor<'_, 'r, State, Action, Root>, 
        reservation_range: item::id::Range,
        input: PlayerInit::In,
    ) -> Result<player::init::Complete<'r, Action, Root>, RecoverableError<player::init::Error>>
    where 
        State: crate::State,
        Action: crate::Action<State = State>,
        PlayerInit: player::Init<State = State, Action = Action, Root = Root>, 
    {
        let id = player::Id(self.player_details.len() as u32);
        interactor.context_mut().player = id;

        let init_error = self.init.initialize(&mut interactor, input)
            .map_err(|e| e.with_context(&self.init))
            .err();
        
        let complete = interactor.complete(init_error)?;
        self.player_details.push(Details {
            reservation_range,
        });
        Ok(complete)
    }

    pub(crate) fn revert_add(&mut self) {
        self.player_details.pop()
            .expect("No player to revert");
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = player::Id> {
        (0..self.player_details.len())
            .into_iter()
            .map(|i| player::Id(i as u32))
    }

    pub(crate) fn get(&self, player_id: player::Id) -> &Details {
        self.player_details.get(player_id.0 as usize)
            .expect(&format!("Player {player_id}  does not exist"))
    }
}
