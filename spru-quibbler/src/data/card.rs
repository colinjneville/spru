use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Card {
    face_index: usize,
}

impl Card {
    fn new(face_index: usize) -> Self {
        Self { face_index }
    }

    pub fn face(&self) -> &'static Face {
        &FACES[self.face_index].face
    }

    pub fn all() -> Vec<Card> {
        let mut cards = vec![];
        for (i, face_count) in FACES.iter().enumerate() {
            for _ in 0..face_count.count {
                cards.push(Card { face_index: i });
            }
        }
        cards
    }
    pub(crate) fn get(letters: &[u8]) -> Option<Self> {
        match letters {
            b"QU" => Some(Card::new(26)),
            b"IN" => Some(Card::new(27)),
            b"ER" => Some(Card::new(28)),
            b"CL" => Some(Card::new(29)),
            b"TH" => Some(Card::new(30)),
            &[letter] => Some(Card::new((letter - b'A') as usize)),
            _ => None,
        }
    }

    pub(crate) fn get_matching(first: u8, second: Option<u8>) -> (Self, Option<Self>) {
        let first_card = Card::new((first - b'A') as usize);

        let second_card = match (first, second) {
            (b'Q', Some(b'U')) => Some(Card::new(26)),
            (b'I', Some(b'N')) => Some(Card::new(27)),
            (b'E', Some(b'R')) => Some(Card::new(28)),
            (b'C', Some(b'L')) => Some(Card::new(29)),
            (b'T', Some(b'H')) => Some(Card::new(30)),
            _ => None,
        };

        (first_card, second_card)
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.face().letters)
    }
}

#[derive(Debug)]
pub struct Face {
    pub letters: &'static str,
    pub points: u8,
}

impl Face {
    const fn new(letters: &'static str, points: u8) -> Self {
        Self { letters, points }
    }

    pub fn points(&self) -> u32 {
        self.points as u32
    }
}

pub struct FaceCount {
    face: Face,
    count: u8,
}

impl FaceCount {
    const fn new(letters: &'static str, points: u8, count: u8) -> Self {
        Self {
            face: Face::new(letters, points),
            count,
        }
    }
}

pub static FACES: [FaceCount; 31] = [
    FaceCount::new("A", 2, 10),
    FaceCount::new("B", 8, 2),
    FaceCount::new("C", 8, 2),
    FaceCount::new("D", 5, 4),
    FaceCount::new("E", 2, 12),
    FaceCount::new("F", 6, 2),
    FaceCount::new("G", 6, 4),
    FaceCount::new("H", 7, 2),
    FaceCount::new("I", 2, 8),
    FaceCount::new("J", 13, 2),
    FaceCount::new("K", 8, 2),
    FaceCount::new("L", 3, 4),
    FaceCount::new("M", 5, 2),
    FaceCount::new("N", 5, 6),
    FaceCount::new("O", 2, 8),
    FaceCount::new("P", 6, 2),
    FaceCount::new("Q", 15, 2),
    FaceCount::new("R", 5, 6),
    FaceCount::new("S", 3, 4),
    FaceCount::new("T", 3, 6),
    FaceCount::new("U", 4, 6),
    FaceCount::new("V", 11, 2),
    FaceCount::new("W", 10, 2),
    FaceCount::new("X", 12, 2),
    FaceCount::new("Y", 4, 4),
    FaceCount::new("Z", 14, 2),
    FaceCount::new("QU", 9, 2),
    FaceCount::new("IN", 7, 2),
    FaceCount::new("ER", 7, 2),
    FaceCount::new("CL", 10, 2),
    FaceCount::new("TH", 9, 2),
];
