
use crate::{action, Interactor, item::{self}, player};


#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InitializeError<PlayerInitError> {
    Lookup(item::lookup::canonical::Error),
    Init(PlayerInitError),
}

impl<PlayerInitError> From<player::init::Error<PlayerInitError>> for InitializeError<PlayerInitError> {
    fn from(value: player::init::Error<PlayerInitError>) -> Self {
        match value {
            player::init::Error::Lookup(e) => Self::Lookup(e),
            player::init::Error::Init(e) => Self::Init(e),
        }
    }
}

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Details {
    reservation_range: item::id::Range,
}

#[derive(Debug)]
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

    // pub(crate) fn initialize<Data, Action, Root>(
    //     &self, 
    //     interactor: &mut Interactor<item::lookup::Canonical<Data>, Action, Root>, 
    //     input: PlayerInit::In
    // ) -> Result<PlayerInit::Out, InitializeError<PlayerInit::Error>>
    // where 
    //     Action: crate::Action<item::lookup::Canonical<Data>>,
    //     Data: crate::State<item::lookup::Canonical<Data>>,
    //     PlayerInit: crate::Init<Data, Action, Root>, 
    // {
    //     let output = self.init.initialize(interactor, input)?;
    //     Ok(output)
    // }

    pub(crate) fn add<State, Action, Root> (
        &mut self, 
        interactor: &mut player::init::Interactor<State, Action, Root>, 
        reservation_range: item::id::Range,
        input: PlayerInit::In,
    ) -> player::init::Result<player::Id>
    where 
        Action: crate::Action<item::lookup::Canonical<State>, Undo = Action>,
        State: crate::State<item::lookup::Canonical<State>>,
        PlayerInit: player::Init<State = State, Action = Action, Root = Root>, 
    {
        let id = player::Id(self.player_details.len());
        interactor.context_mut().player = id;
        let result = 'result: {
            if let Err(err) = self.init.initialize(interactor, input) {
                let err = match err {
                    player::init::Error::Lookup(error) => action::Error::Lookup(error),
                    player::init::Error::Init(_) => None,
                };
                break 'result Err(err);
            }

            if let Err(err) = interactor.flush() {
                break 'result Err(Some(err));
            }

            Ok(())
        };

        match result {
            Ok(()) => {
                let complete = interactor.complete();

            }
            Err(err) => {
                interactor.flush()
            }
        }
        
        self.player_details.push(Details {
            reservation_range,
        });
        
        Ok(id)
    }

    pub(crate) fn revert_add(&mut self) {
        self.player_details.pop()
            .expect("No player to revert");
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = player::Id> {
        (0..self.player_details.len())
            .into_iter()
            .map(|i| player::Id(i))
    }
}
