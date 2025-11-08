use std::fmt;

use crate::data;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Play {
    words: Vec<Vec<data::Card>>,
    unused: Vec<data::Card>,
}

impl Play {
    pub(crate) fn new(words: Vec<Vec<data::Card>>, unused: Vec<data::Card>) -> Self {
        Self { words, unused }
    }

    pub fn is_played(&self) -> bool {
        !(self.words.is_empty() && self.unused.is_empty())
    }

    pub fn is_full(&self) -> bool {
        self.unused.is_empty()
    }

    pub fn base_score(&self) -> u32 {
        let mut score = 0;

        for card in self.words.iter().flatten() {
            score += card.face().points();
        }

        for card in &self.unused {
            score = score.saturating_sub(card.face().points());
        }

        score
    }

    pub fn words(&self) -> impl Iterator<Item = &[data::Card]> {
        self.words.iter().map(Vec::as_slice)
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn max_word_len(&self) -> usize {
        self.words
            .iter()
            .map(|w| w.iter().fold(0, |n, c| n + c.face().letters.len()))
            .max()
            .unwrap_or_default()
    }

    pub fn unused(&self) -> &[data::Card] {
        self.unused.as_slice()
    }
}

impl fmt::Display for Play {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for word in &self.words {
            for card in word {
                write!(f, "{card}")?;
            }
            write!(f, " ")?;
        }

        if !self.unused.is_empty() {
            write!(f, "(")?;
            for card in &self.unused {
                write!(f, "{card}")?;
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}
