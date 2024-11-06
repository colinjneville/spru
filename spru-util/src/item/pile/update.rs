use std::marker::PhantomData;

use perfect_derive::perfect_derive;
use spru::Serial;

use crate::Strictness;
use super::*;


#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = PopTop<T>)]
pub struct PushTop<T> {
    item: T,
}

impl<T> PushTop<T> {
    pub fn new(item: T) -> Self {
        Self { item }
    }
}

impl<T> spru::Action for PushTop<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        input.items.push_front(self.item.clone());
        Ok(PopTop::new(Strictness::AllOrError))
    }
}

#[perfect_derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = PushTop<T>, Error = error::Pop)]
pub struct PopTop<T> {
    strictness: Strictness,
    _p: PhantomData<fn() -> T>,
}

impl<T> spru::Action for PopTop<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(match input.items.pop_front() {
            Some(item) => Some(PushTop::new(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(Self::Error::Empty)?,
            }
        })
    }
}

impl<T> PopTop<T> {
    pub fn new(strictness: Strictness) -> Self {
        Self { strictness, _p: PhantomData }
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = PopBottom<T>)]
pub struct PushBottom<T> {
    item: T,
}

impl<T> PushBottom<T> {
    pub fn new(item: T) -> Self {
        Self {item }
    }
}

impl<T> spru::Action for PushBottom<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        input.items.push_back(self.item.clone());
        Ok(PopBottom::new(Strictness::AllOrError))
    }

    
}

#[perfect_derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = PushBottom<T>, Error = error::Pop)]
pub struct PopBottom<T> {
    strictness: Strictness,
    _p: PhantomData<T>,
}

impl<T> spru::Action for PopBottom<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(match input.items.pop_back() {
            Some(item) => Some(PushBottom::new(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(Self::Error::Empty)?,
        }})
    }
}

impl<T> PopBottom<T> {
    pub fn new(strictness: Strictness) -> Self {
        Self { strictness, _p: PhantomData }
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update]
pub struct Shuffle<T> {
    seed: u64,
    undo: bool,
    _p: PhantomData<T>,
}

impl<T> spru::Action for Shuffle<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        use rand::{Rng, SeedableRng};

        let mut rng = crate::Rng::seed_from_u64(self.seed);

        // Based on the soon-to-be-replaced rand 0.85 implementation
        fn gen_index(rng: &mut crate::Rng, ubound: usize) -> usize {
            if ubound <= (core::u32::MAX as usize) {
                rng.gen_range(0..ubound as u32) as usize
            } else {
                rng.gen_range(0..ubound)
            }
        }

        let mut deferred = self.undo.then_some(vec![]);

        for i in (1..input.items.len()).rev() {
            let index = gen_index(&mut rng, i + 1);
            // invariant: elements with index > i have been locked in place.
            match &mut deferred {
                Some(deferred) => deferred.push(index),
                None => input.items.swap(i, index),
            }
        }

        if let Some(deferred) = deferred {
            for (i, index) in (1..input.items.len()).into_iter().zip(deferred.into_iter().rev()) {
                input.items.swap(i, index);
            }
        }

        Ok(self.invert())
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