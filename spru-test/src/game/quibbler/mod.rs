use std::convert::Infallible;

use spru::item::IdT;

pub enum ItemCatalog {

}

pub enum ActionCatalog {

}

pub struct Root {
    pub turn_order: IdT<spru_util::item::Rotating<spru::player::Id>>,
}

pub struct PlayerData;

pub struct PlayerInit;

impl spru::init::Base for PlayerInit {
    type In;
    type Out = ();
    type Error = Infallible;
}

impl spru::Init<ItemCatalog, ActionCatalog, Root> for PlayerInit {
    fn initialize(&self, interactor: &mut spru::interaction::Interactor<spru::item::lookup::Canonical<ItemCatalog>, ActionCatalog, Root>, input: Self::In) 
        -> Result<Self::Out, spru::init::Error<Self::Error>> 
    {
        spru::server::Server::add_player(&mut self, arg)
        todo!()
    }
}

