use std::mem;

use derive_where::derive_where;

use rand::{seq::IndexedRandom, Rng};
use spru::error::AnyResult;
use tagset::tagset;
use telety::telety;

use crate::verbatim;

#[derive(Debug)]
pub struct State<Die: self::Die> {
    die: Die,
    current_face: Die::Face,
}

impl<Die: self::Die> State<Die> {
    pub fn current_face(&self) -> &Die::Face {
        &self.current_face
    }
}

pub fn create<Die: self::Die>(die: Die) -> Create<Die> {
    let mut mock_rng = rand::rngs::mock::StepRng::new(0, 0);
    let current_face = die.roll(&mut mock_rng);
    verbatim::create(State {
        die,
        current_face,
    })
}

pub fn roll<Die: self::Die, R: rand::Rng>(state: &State<Die>, rng: &mut R) -> SetFace<Die> {
    let face = state.die.roll(rng);
    set_face(face)
}

pub fn set_face<Die: self::Die>(face: Die::Face) -> SetFace<Die> {
    SetFace {
        face,
    }
}

pub fn destroy<Die: self::Die>() -> Destroy<Die> {
    verbatim::destroy()
}

#[telety(crate::die)]
#[tagset(Create<Die>)]
#[tagset(SetFace<Die>)]
#[tagset(Destroy<Die>)]
pub struct Actions<Die: self::Die>;

pub type Create<Die> = verbatim::Create<State<Die>>;

#[derive_where(Debug, Clone; Die::Face)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct SetFace<Die: self::Die> {
    face: Die::Face,
}

impl<Die: self::Die<Face: Clone>> spru::action::Update for SetFace<Die> {
    type T = State<Die>;
    type Undo = Self;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let face = mem::replace(&mut value.current_face, self.face.clone());

        Ok(Self {
            face,
        })
    }
}

pub type Destroy<Die> = verbatim::Destroy<State<Die>>;

pub trait Die {
    type Face;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face;
}

/// A die with one of the first 'N' natural numbers as the faces, 
/// such as the standard 6-sided die, or a d20.
#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DN<T>(T);

impl<T> DN<T> {
    pub fn new(n: T) -> Self {
        Self(n)
    }

    pub fn n(&self) -> &T {
        &self.0
    }
}

impl<T> Die for DN<T>
where T: num_traits::Zero
    + rand::distr::uniform::SampleUniform 
    + std::cmp::PartialOrd
    + Clone
{
    type Face = T;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face {
        rng.random_range(T::zero()..self.0.clone())
    }
}

/// A die with individually specified faces of type T
#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Custom<T>(Vec<T>);

impl<T> Custom<T> {
    pub fn new(faces: Vec<T>) -> Self {
        assert!(!faces.is_empty());
        Self(faces)
    }
}

impl<T: Clone> Die for Custom<T> {
    type Face = T;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face {
        self.0.choose(rng).unwrap().clone()
    }
}