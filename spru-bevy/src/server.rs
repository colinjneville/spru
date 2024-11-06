use std::ops;

use bevy::prelude::*;

use crate::item;

#[derive(Resource)]
pub struct BevyServer<GameRoot, PlayerInit: spru::init::Base> {
    server: spru::Server<GameRoot, PlayerInit>,
}

impl<GameRoot, PlayerInit: spru::init::Base> BevyServer<GameRoot, PlayerInit> {
    pub fn new<'l, GameInit>(lookup: &mut item::BevyLookupMut<'l>, game_init: GameInit, input: GameInit::In, player_init: PlayerInit) -> Result<Self, spru::error::LookupInteractionError<item::lookup::BevyError, GameInit::Error>> 
    where
        PlayerInit::ActionCatalog: spru::action::catalog::Apply<item::BevyLookupMut<'l>, Undo = PlayerInit::ActionCatalog>,
        GameInit: spru::Init<PlayerInit::Out, item::BevyLookupMut<'l>, Out = GameRoot, ActionCatalog = PlayerInit::ActionCatalog>,
    {
        let server = spru::Server::new(lookup, game_init, input, player_init)?;
        Ok(Self {
            server,
        })
    }
    
    // pub fn add_player<'l>(&mut self, lookup: &mut item::Lookup<'l>, input: PlayerInit::In) -> Result<spru::player::Id, spru::error::LookupInteractionError<item::LookupError, PlayerInit::Error>> 
    // where PlayerInit: spru::Init<<PlayerInit as spru::init::Base>::Out, item::Lookup<'l>, ActionCatalog: spru::action::Catalog<item::Lookup<'l>>>,
    // {
    //     self.server.add_player(lookup, input)
    // }
}

impl<GameRoot, PlayerInit: spru::init::Base> ops::Deref for BevyServer<GameRoot, PlayerInit> {
    type Target = spru::Server<GameRoot, PlayerInit>;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}

impl<GameRoot, PlayerInit: spru::init::Base> ops::DerefMut for BevyServer<GameRoot, PlayerInit> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.server
    }
}