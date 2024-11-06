pub mod update;

use amass::amass_telety;
use update::{AddCard, RemoveCard, RemoveCardError};

use crate::data::Card;

#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Hand {
    cards: Vec<Card>,
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::Actions)]
#[actions(error = Error)]
#[amass_telety(crate::component::hand)]
pub enum Actions {
    Create(Create),
    AddCard(AddCard),
    RemoveCard(RemoveCard),
    Destroy(Destroy),
}

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::component::hand)]
pub enum Error {
    RemoveCard(RemoveCardError),
}

#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Create {
    cards: Vec<Card>,
}

impl spru::Action for Create {
    spru::create!(T = Hand, Undo = Destroy);
    
    fn apply<'i>(&self, _input: Self::In<'i>) -> Result<impl Into<spru::action::Output<Self::Undo, Self::Out>>, Self::Error> {
        Ok((Destroy, Hand { cards: self.cards.clone() }))
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Destroy;

impl spru::Action for Destroy {
    spru::destroy!(T = Hand, Undo = Create);
    
    fn apply<'i>(&self, input: Self::In<'i>) -> Result<impl Into<spru::action::Output<Self::Undo, Self::Out>>, Self::Error> {
        Ok(Create { cards: input.cards })
    }
}
