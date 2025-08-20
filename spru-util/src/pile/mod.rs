pub mod error;

use std::{collections::VecDeque, marker::PhantomData, mem};

use amass::amass_telety;
use derive_where::derive_where;
use spru::Serial;
use tagset::tagset;
use telety::telety;

use crate::{verbatim, Strictness};

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct State<T> {
    /// front/top -> back/bottom
    items: VecDeque<T>,
}

impl<T> State<T> {
    /// Iterate items from top to bottom
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.into_iter()
    }

    pub fn top(&self) -> Option<&T> {
        self.items.front()
    }

    pub fn bottom(&self) -> Option<&T> {
        self.items.back()
    }
}

impl<'i, T> IntoIterator for &'i State<T> {
    type Item = &'i T;
    type IntoIter = std::collections::vec_deque::Iter<'i, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

pub fn create<T>(items: impl IntoIterator<Item = T>) -> Create<T> {
    verbatim::create(State {
        items: items.into_iter().collect(),
    })
}

pub fn default<T>() -> Create<T> {
    create([])
}

pub fn destroy<T>() -> Destroy<T> {
    verbatim::destroy()
}

pub fn update<T>(items: impl IntoIterator<Item = T>) -> Update<T> {
    verbatim::update(State {
        items: items.into_iter().collect(),
    })
}

pub fn shuffle<T, R: rand::Rng>(rng: &mut R) -> Shuffle<T> {
    let seed = rng.random();
    Shuffle {
        seed,
        undo: false,
        _p: PhantomData,
    }
}

pub fn push_top<T>(item: T) -> PushTop<T> {
    PushTop {
        item,
    }
}

pub fn try_pop_top<T>() -> PopTop<T> {
    PopTop {
        strictness: Strictness::BestEffort,
        _p: PhantomData,
    }
}

pub fn pop_top<T>() -> PopTop<T> {
    PopTop {
        strictness: Strictness::AllOrError,
        _p: PhantomData,
    }
}

pub fn push_bottom<T>(item: T) -> PushBottom<T> {
    PushBottom { item }
}

pub fn try_pop_bottom<T>() -> PopBottom<T> {
    PopBottom { 
        strictness: Strictness::BestEffort,
        _p: PhantomData
    }
}

pub fn pop_bottom<T>() -> PopBottom<T> {
    PopBottom {
        strictness: Strictness::AllOrError,
        _p: PhantomData,
    }
}

pub fn remove<T>(index: usize) -> Remove<T> {
    Remove {
        index,
        _p: PhantomData,
    }
}

pub fn clear<T>() -> Clear<T> {
    Clear {
        _p: PhantomData,
    }
}

#[telety(crate::pile)]
#[tagset(Create<T>)]
#[tagset(Destroy<T>)]
#[tagset(Update<T>)]
#[tagset(PushTop<T>)]
#[tagset(PushBottom<T>)]
#[tagset(PopTop<T>)]
#[tagset(PopBottom<T>)]
#[tagset(Insert<T>)]
#[tagset(Remove<T>)]
#[tagset(Shuffle<T>)]
#[tagset(Clear<T>)]
#[tagset(reserved(..32))]
pub struct Actions<T>;

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::pile)]
pub enum Error {
    Pop(error::Pop),
    Insert(error::Insert),
    Remove(error::Remove),
}

pub type Create<T> = verbatim::Create<State<T>>;

pub type Destroy<T> = verbatim::Destroy<State<T>>;

pub type Update<T> = verbatim::Update<State<T>>;


#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct PushTop<T> {
    item: T,
}

impl<T> spru::action::Update for PushTop<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopTop<T>;
    type Error = std::convert::Infallible;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        value.items.push_front(self.item.clone());
        Ok(Some(pop_top()))
    }
}

#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct PopTop<T> {
    strictness: Strictness,
    _p: PhantomData<fn() -> T>,
}

impl<T> spru::action::Update for PopTop<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PushTop<T>;
    type Error = error::Pop;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        Ok(match value.items.pop_front() {
            Some(item) => Some(push_top(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(error::Pop::Empty)?,
            }
        })
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct PushBottom<T> {
    item: T,
}

impl<T> spru::action::Update for PushBottom<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopBottom<T>;
    type Error = std::convert::Infallible;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        value.items.push_back(self.item.clone());
        Ok(Some(pop_bottom()))
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct PopBottom<T> {
    strictness: Strictness,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for PopBottom<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PushBottom<T>;
    type Error = error::Pop;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        Ok(match value.items.pop_back() {
            Some(item) => Some(push_bottom(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(Self::Error::Empty)?,
        }})
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct Shuffle<T> {
    seed: u64,
    undo: bool,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Shuffle<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = Self;
    type Error = std::convert::Infallible;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        use rand::{Rng, SeedableRng};

        let mut rng = crate::Rng::seed_from_u64(self.seed);

        // Based on the soon-to-be-replaced rand 0.85 implementation
        fn gen_index(rng: &mut crate::Rng, ubound: usize) -> usize {
            if ubound <= (core::u32::MAX as usize) {
                rng.random_range(0..ubound as u32) as usize
            } else {
                rng.random_range(0..ubound)
            }
        }

        let mut deferred = self.undo.then_some(vec![]);

        for i in (1..value.items.len()).rev() {
            let index = gen_index(&mut rng, i + 1);
            // invariant: elements with index > i have been locked in place.
            match &mut deferred {
                Some(deferred) => deferred.push(index),
                None => value.items.swap(i, index),
            }
        }

        if let Some(deferred) = deferred {
            for (i, index) in (1..value.items.len()).into_iter().zip(deferred.into_iter().rev()) {
                value.items.swap(i, index);
            }
        }

        Ok(Some(self.invert()))
    }
}

impl<T> Shuffle<T> {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            undo: false,
            _p: PhantomData,
        }
    }

    fn invert(&self) -> Self {
        Self {
            seed: self.seed, 
            undo: !self.undo, 
            _p: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
pub struct Insert<T> {
    index: usize,
    element: T,
}

impl<T: Clone> spru::action::Update for Insert<T> {
    type T = State<T>;
    type Undo = Remove<T>;
    type Error = error::Insert;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let index = self.index;
        if index <= value.items.len() {
            value.items.insert(index, self.element.clone());
            Ok(Some(Remove {
                index,
                _p: PhantomData,
            }))
        } else {
            Err(error::Insert::Index(index, value.items.len()))
        }
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct Remove<T> {
    index: usize,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Remove<T> {
    type T = State<T>;
    type Undo = Insert<T>;
    type Error = error::Remove;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let index = self.index;
        if let Some(element) = value.items.remove(index) {
            Ok(Some(Insert {
                index,
                element,
            }))
        } else {
            Err(error::Remove::Index(index, value.items.len()))
        }
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct Clear<T> {
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Clear<T> {
    type T = State<T>;
    type Undo = Update<T>;
    type Error = std::convert::Infallible;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let items = mem::take(&mut value.items);
        Ok(Some(update(items)))
    }
}


#[cfg(test)]
mod test {
    use rand::rng;
    use spru::action::{Create as _, Update as _};

    use super::*;

    #[test]
    fn test_shuffle() {
        let create = create([1, 2, 3, 4, 5, 6, 7, 8]);
        let (mut pile, _) = create.create()
            .unwrap();

        shuffle(&mut rng()).update(&mut pile)
            .unwrap();

        let orig_order = pile.items.clone();

        let undo = shuffle(&mut rng()).update(&mut pile)
            .unwrap();

        assert_ne!(pile.items, orig_order);

        undo.unwrap().update(&mut pile)
            .unwrap();

        assert_eq!(pile.items, orig_order);
    }
}