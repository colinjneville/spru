use std::mem;

use derive_where::derive_where;

use rand::{Rng, seq::IndexedRandom};
use spru::common::error::AnyResult;
use tagset::tagset;
use telety::telety;

use crate::cloned;

#[derive(Debug)]
pub struct Die<D: self::DieKind> {
    die: D,
    current_face: D::Face,
}

impl<D: self::DieKind> Die<D> {
    pub fn current_face(&self) -> &D::Face {
        &self.current_face
    }
}

pub fn create<D: self::DieKind>(die: D) -> Create<D> {
    let mut mock_rng = rand::rngs::mock::StepRng::new(0, 0);
    let current_face = die.roll(&mut mock_rng);
    cloned::create(Die { die, current_face })
}

/// Sets the die to a random face. Should not be used in interactions, as the interaction
/// will be rejected if the outcome does not match on the server.
pub fn roll<D: self::DieKind, R: rand::Rng>(state: &Die<D>, rng: &mut R) -> SetFace<D> {
    let face = state.die.roll(rng);
    set_face(face)
}

pub fn set_face<D: self::DieKind>(face: D::Face) -> SetFace<D> {
    SetFace { face }
}

pub fn destroy<D: self::DieKind>() -> Destroy<D> {
    cloned::destroy()
}

#[telety(crate::die)]
#[tagset(Create<D>)]
#[tagset(SetFace<D>)]
#[tagset(Destroy<D>)]
pub struct Actions<D: self::DieKind>;

pub type Create<D> = cloned::Create<Die<D>>;

#[derive_where(Debug, Clone; D::Face)]
#[derive(serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct SetFace<D: self::DieKind> {
    face: D::Face,
}

impl<D: self::DieKind<Face: Clone>> spru::action::Update for SetFace<D> {
    type T = Die<D>;
    type Undo = Self;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let face = mem::replace(&mut value.current_face, self.face.clone());

        Ok(Self { face })
    }
}

pub type Destroy<D> = cloned::Destroy<Die<D>>;

pub trait DieKind {
    type Face;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face;
}

/// A die with one of the first 'N' natural numbers as the faces,
/// such as the standard 6-sided die, or a d20.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DN<T>(T);

impl<T> DN<T> {
    pub fn new(n: T) -> Self {
        Self(n)
    }

    pub fn n(&self) -> &T {
        &self.0
    }
}

impl<T> DieKind for DN<T>
where
    T: num_traits::Zero + rand::distr::uniform::SampleUniform + std::cmp::PartialOrd + Clone,
{
    type Face = T;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face {
        rng.random_range(T::zero()..self.0.clone())
    }
}

/// A die with individually specified faces of type T
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Custom<T>(Vec<T>);

impl<T> Custom<T> {
    pub fn new(faces: Vec<T>) -> Self {
        assert!(!faces.is_empty());
        Self(faces)
    }
}

impl<T: Clone> DieKind for Custom<T> {
    type Face = T;

    fn roll<R: Rng>(&self, rng: &mut R) -> Self::Face {
        self.0.choose(rng).unwrap().clone()
    }
}
