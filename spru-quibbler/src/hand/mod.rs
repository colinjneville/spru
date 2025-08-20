use amass::amass_telety;
use spru_util::verbatim;
use tagset::tagset;
use telety::telety;

use crate::data::Card;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    cards: Vec<Card>,
}

#[telety(crate::hand)]
#[tagset(verbatim::Create<State>)]
#[tagset(AddCard)]
#[tagset(RemoveCard)]
#[tagset(verbatim::Destroy<State>)]
#[tagset(reserved(..8))]
pub struct Actions;

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::hand)]
pub enum Error {
    RemoveCard(RemoveCardError),
}


#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct AddCard {
    card: Card,
}

impl AddCard {
    pub fn new(card: Card) -> Self {
        Self {
            card,
        }
    }
}

impl spru::action::Update for AddCard {
    type T = State;
    type Undo = RemoveCard;
    type Error = std::convert::Infallible;
    
    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        value.cards.push(self.card.clone());
        Ok(Some(RemoveCard::new(self.card.clone())))
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct RemoveCard {
    card: Card,
}

impl RemoveCard {
    pub fn new(card: Card) -> Self {
        Self {
            card,
        }
    }
}

impl spru::action::Update for RemoveCard {
    type T = State;
    type Undo = AddCard;
    type Error = RemoveCardError;
    
    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        for (i, c) in value.cards.iter().enumerate().rev() {
            if &self.card == c {
                value.cards.remove(i);
                return Ok(Some(AddCard::new(self.card.clone())));
            }
        }
        return Err(RemoveCardError::CardDoesNotExist);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(thiserror::Error)]
pub enum RemoveCardError {
    #[error("Card is not in hand.")]
    CardDoesNotExist,
}
