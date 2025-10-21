
use crate::{error::RecoverableResult, item::{self}, player};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Details {
    reservation_range: item::id::Range,
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

    pub(crate) fn add<'r, State, Action, Root> (
        &mut self, 
        mut interactor: player::init::Interactor<'_, 'r, State, Action, Root>, 
        reservation_range: item::id::Range,
        input: PlayerInit::In,
    ) -> RecoverableResult<player::init::Complete<'r, Action, Root>, player::init::Error>
    where 
        State: crate::State,
        Action: crate::Action<State = State>,
        // State: crate::State<item::lookup::Canonical<State>>,
        PlayerInit: player::Init<State = State, Action = Action, Root = Root>, 
    {
        let id = player::Id(self.player_details.len() as u32);
        interactor.context_mut().player = id;

        let init_error = self.init.initialize(&mut interactor, input).err();
        
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
}
