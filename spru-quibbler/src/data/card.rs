use std::fmt;

use spru_script::script;

pub type Card = spru_script::Wrap<CardImpl>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[script(state = false, include = [Methods], derive = [Eq])]
pub struct CardImpl {
    face_index: usize,
}

#[script(state = false, partial = Methods)]
impl CardImpl {
    #[get]
    fn letters(&self) -> String {
        self.face().letters_str().to_string()
    }

    #[get]
    fn points(&self) -> u32 {
        self.face().points()
    }
}

impl CardImpl {
    fn new(face_index: usize) -> Self {
        Self { face_index }
    }

    pub fn face(&self) -> &'static Face {
        &FACES[self.face_index].face
    }

    #[allow(dead_code)]
    pub(crate) fn get(letters: &[u8]) -> Option<Self> {
        match letters {
            b"QU" => Some(Self::new(26)),
            b"IN" => Some(Self::new(27)),
            b"ER" => Some(Self::new(28)),
            b"CL" => Some(Self::new(29)),
            b"TH" => Some(Self::new(30)),
            &[letter] => Some(Self::new((letter - b'A') as usize)),
            _ => None,
        }
    }

    pub(crate) fn get_matching(first: u8, second: Option<u8>) -> (Self, Option<Self>) {
        let first_card = Self::new((first - b'A') as usize);

        let second_card = match (first, second) {
            (b'Q', Some(b'U')) => Some(Self::new(26)),
            (b'I', Some(b'N')) => Some(Self::new(27)),
            (b'E', Some(b'R')) => Some(Self::new(28)),
            (b'C', Some(b'L')) => Some(Self::new(29)),
            (b'T', Some(b'H')) => Some(Self::new(30)),
            _ => None,
        };

        (first_card, second_card)
    }
}

impl fmt::Display for CardImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", str::from_utf8(self.face().letters).unwrap())
    }
}

pub fn all() -> Vec<Card> {
    let mut cards = vec![];
    for (i, face_count) in FACES.iter().enumerate() {
        for _ in 0..face_count.count {
            cards.push(Card::new(CardImpl { face_index: i }));
        }
    }
    cards
}

#[derive(Debug)]
pub struct Face {
    pub letters: &'static [u8],
    pub points: u8,
}

impl Face {
    const fn new(letters: &'static [u8], points: u8) -> Self {
        Self { letters, points }
    }

    pub fn points(&self) -> u32 {
        self.points as u32
    }

    pub fn letters_str(&self) -> &'static str {
        str::from_utf8(self.letters).unwrap()
    }
}

pub struct FaceCount {
    face: Face,
    count: u8,
}

impl FaceCount {
    const fn new(letters: &'static [u8], points: u8, count: u8) -> Self {
        Self {
            face: Face::new(letters, points),
            count,
        }
    }
}

pub static FACES: [FaceCount; 31] = [
    FaceCount::new(b"A", 2, 10),
    FaceCount::new(b"B", 8, 2),
    FaceCount::new(b"C", 8, 2),
    FaceCount::new(b"D", 5, 4),
    FaceCount::new(b"E", 2, 12),
    FaceCount::new(b"F", 6, 2),
    FaceCount::new(b"G", 6, 4),
    FaceCount::new(b"H", 7, 2),
    FaceCount::new(b"I", 2, 8),
    FaceCount::new(b"J", 13, 2),
    FaceCount::new(b"K", 8, 2),
    FaceCount::new(b"L", 3, 4),
    FaceCount::new(b"M", 5, 2),
    FaceCount::new(b"N", 5, 6),
    FaceCount::new(b"O", 2, 8),
    FaceCount::new(b"P", 6, 2),
    FaceCount::new(b"Q", 15, 2),
    FaceCount::new(b"R", 5, 6),
    FaceCount::new(b"S", 3, 4),
    FaceCount::new(b"T", 3, 6),
    FaceCount::new(b"U", 4, 6),
    FaceCount::new(b"V", 11, 2),
    FaceCount::new(b"W", 10, 2),
    FaceCount::new(b"X", 12, 2),
    FaceCount::new(b"Y", 4, 4),
    FaceCount::new(b"Z", 14, 2),
    FaceCount::new(b"QU", 9, 2),
    FaceCount::new(b"IN", 7, 2),
    FaceCount::new(b"ER", 7, 2),
    FaceCount::new(b"CL", 10, 2),
    FaceCount::new(b"TH", 9, 2),
];
