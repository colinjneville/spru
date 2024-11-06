use crate::data::Card;
use super::*;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
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

impl spru::Action for AddCard {
    spru::update!(T = Hand, Undo = RemoveCard);
    
    fn apply<'i>(&self, input: Self::In<'i>) -> Result<impl Into<spru::action::Output<Self::Undo, Self::Out>>, Self::Error> {
        input.cards.push(self.card.clone());
        Ok(RemoveCard::new(self.card.clone()))
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
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

impl spru::Action for RemoveCard {
    spru::update!(T = Hand, Undo = AddCard, Error = RemoveCardError);
    
    fn apply<'i>(&self, input: Self::In<'i>) -> Result<impl Into<spru::action::Output<Self::Undo, Self::Out>>, Self::Error> {
        for (i, c) in input.cards.iter().enumerate().rev() {
            if &self.card == c {
                input.cards.remove(i);
                return Ok(AddCard::new(self.card.clone()));
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