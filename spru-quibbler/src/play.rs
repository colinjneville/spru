use std::{collections::HashMap, fmt, mem};

use spru_script::{Wrap, script};
use spru_util::pile;

use crate::data;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[script(state = false, include = [Methods])]
pub struct Play {
    #[get]
    words: Vec<Vec<data::Card>>,
    #[get]
    unused: Vec<data::Card>,
}

#[script(state = false, partial = Methods)]
impl Play {
    #[function(name = new)]
    fn _new(words: Vec<Vec<data::Card>>, unused: Vec<data::Card>) -> Wrap<Play> {
        Wrap::new(Self::new(words, unused))
    }

    #[get(name = is_played)]
    fn _is_played(&self) -> bool {
        self.is_played()
    }

    #[get(name = is_full)]
    fn _is_full(&self) -> bool {
        self.is_full()
    }

    #[get(name = base_score)]
    fn _base_score(&self) -> u32 {
        self.base_score()
    }

    #[get(name = words)]
    fn _words(&self) -> Vec<Vec<data::Card>> {
        self.words()
            .map(<[data::Card]>::to_vec)
            .collect()
    }

    #[get(name = word_count)]
    fn _word_count(&self) -> usize {
        self.words.len()
    }

    #[get(name = max_word_len)]
    fn _max_word_len(&self) -> usize {
        self.max_word_len()
    }

    #[get(name = unused)]
    fn _unused(&self) -> Vec<data::Card> {
        self.unused().to_vec()
    }

    #[function]
    fn check_word(word: String) -> bool {
        // TODO script macro does not currently handle mut in arg patterns correctly
        let mut word = word;
        word.make_ascii_lowercase();
        wordnik_list::word_exists(&word)
    }
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

    pub fn parsed(hand: &pile::Pile<data::Card>, s: &[u8]) -> Result<Self, u8> {
        let mut remaining_cards = HashMap::new();
        for card in hand {
            *remaining_cards.entry(&card.0).or_insert(0) += 1;
        }

        let mut words = vec![];
        let mut current_word = vec![];

        let mut iter = s.iter().copied().peekable();
        while let Some(first) = iter.next() {
            if first == b' ' {
                if !current_word.is_empty() {
                    words.push(mem::take(&mut current_word));
                }
            } else {
                let second = iter.peek().copied();
                let (first_card, second_card) = data::CardImpl::get_matching(first, second);
                if let Some(second_card) = second_card
                    && let Some(card_count) = remaining_cards.get_mut(&second_card)
                    && *card_count > 0
                {
                    *card_count -= 1;
                    current_word.push(data::Card::new(second_card.clone()));
                    // Skip next letter, as we used a double letter card
                    iter.next();

                    continue;
                }

                if let Some(card_count) = remaining_cards.get_mut(&first_card)
                    && *card_count > 0
                {
                    *card_count -= 1;
                    current_word.push(data::Card::new(first_card.clone()));

                    continue;
                }

                // No cards for this letter(s)
                return Err(first);
            }
        }

        if !current_word.is_empty() {
            words.push(mem::take(&mut current_word));
        }

        let mut unused = vec![];
        for (card, count) in remaining_cards {
            for _ in 0..count {
                unused.push(data::Card::new(card.clone()));
            }
        }

        Ok(Self::new(words, unused))
    }
}

impl fmt::Display for Play {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for word in &self.words {
            for card in word {
                write!(f, "{}", **card)?;
            }
            write!(f, " ")?;
        }

        if !self.unused.is_empty() {
            write!(f, "(")?;
            for card in &self.unused {
                write!(f, "{}", **card)?;
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}
