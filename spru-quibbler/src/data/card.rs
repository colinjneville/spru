#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Card {
    face_index: usize,
}

impl Card {
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
}

pub struct Face {
    pub letters: &'static str,
    pub points: u8,
}

impl Face {
    const fn new(letters: &'static str, points: u8) -> Self {
        Self {
            letters,
            points,
        }
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
            face: Face::new(letters, count),
            count,
        }
    }
}

pub static FACES: [FaceCount; 31] = [
    FaceCount::new("A", 0, 10),
    FaceCount::new("B", 0, 2),
    FaceCount::new("C", 0, 2),
    FaceCount::new("D", 0, 4),
    FaceCount::new("E", 0, 12),
    FaceCount::new("F", 0, 2),
    FaceCount::new("G", 0, 4),
    FaceCount::new("H", 0, 2),
    FaceCount::new("I", 0, 8),
    FaceCount::new("J", 0, 2),
    FaceCount::new("K", 0, 2),
    FaceCount::new("L", 0, 4),
    FaceCount::new("M", 0, 2),
    FaceCount::new("N", 0, 6),
    FaceCount::new("O", 0, 8),
    FaceCount::new("P", 0, 2),
    FaceCount::new("Q", 0, 2),
    FaceCount::new("R", 0, 6),
    FaceCount::new("S", 0, 4),
    FaceCount::new("T", 0, 6),
    FaceCount::new("U", 0, 6),
    FaceCount::new("V", 0, 2),
    FaceCount::new("W", 0, 2),
    FaceCount::new("X", 0, 2),
    FaceCount::new("Y", 0, 4),
    FaceCount::new("Z", 0, 2),

    FaceCount::new("QU", 0, 2),
    FaceCount::new("IN", 0, 2),
    FaceCount::new("ER", 0, 2),
    FaceCount::new("CL", 0, 2),
    FaceCount::new("TH", 0, 2),
];