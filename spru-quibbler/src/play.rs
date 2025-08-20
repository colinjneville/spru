use crate::data;


#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Play {
    words: Vec<Vec<data::Card>>,
    unused: Vec<data::Card>,
}

impl Play {
    pub fn is_played(&self) -> bool {
        !(self.words.is_empty() && self.unused.is_empty())
    }

    pub fn base_score(&self) -> u32 {
        let mut score = 0;
        
        for card in self.words.iter().flatten() {
            score += card.face().points();
        }

        for card in &self.unused {
            score.saturating_sub(card.face().points());
        }

        score
    }

    pub fn words(&self) -> impl Iterator<Item = &[data::Card]> {
        self.words.iter()
            .map(Vec::as_slice)
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn max_word_len(&self) -> usize {
        self.words.iter()
            .map(Vec::len)
            .max()
            .unwrap_or_default()
    }
}