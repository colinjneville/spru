pub mod error;

use std::{marker::PhantomData, mem, ops};

use amass::amass_telety;
use derive_where::derive_where;
use spru::{Serial, common::error::AnyResult};
use tagset::tagset;
use telety::telety;

use crate::{Strictness, verbatim};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State<T> {
    /// front/top -> back/bottom
    items: FakeDeVec<T>,
    // spaghetto is broken...
    // items: spaghetto::DeVec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive_where(Default; )]
#[derive(serde::Serialize, serde::Deserialize)]
struct FakeDeVec<T>(Vec<T>);
impl<T> FakeDeVec<T> {
    fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn pop_back(&mut self) -> Option<T> {
        self.pop()
    }

    fn push_front(&mut self, element: T) {
        self.insert(0, element);
    }

    fn push_back(&mut self, element: T) {
        self.push(element);
    }
}
impl<T> std::iter::FromIterator<T> for FakeDeVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}
impl<T> ops::Deref for FakeDeVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> ops::DerefMut for FakeDeVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> State<T> {
    /// Iterate items from top to bottom
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.into_iter()
    }

    pub fn top(&self) -> Option<&T> {
        self.items.first()
    }

    pub fn bottom(&self) -> Option<&T> {
        self.items.last()
    }
}

impl<T> ops::Deref for State<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<'i, T> IntoIterator for &'i State<T> {
    type Item = &'i T;
    type IntoIter = std::slice::Iter<'i, T>;

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
    PushTop { item }
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
        _p: PhantomData,
    }
}

pub fn pop_bottom<T>() -> PopBottom<T> {
    PopBottom {
        strictness: Strictness::AllOrError,
        _p: PhantomData,
    }
}

pub fn pop_top_many<T>(count: usize) -> PopTopMany<T> {
    PopTopMany {
        strictness: Strictness::AllOrError,
        count,
        _p: PhantomData,
    }
}

pub fn try_pop_top_many<T>(count: usize) -> PopTopMany<T> {
    PopTopMany {
        strictness: Strictness::AllOrError,
        count,
        _p: PhantomData,
    }
}

pub fn pop_bottom_many<T>(count: usize) -> PopTopMany<T> {
    PopTopMany {
        strictness: Strictness::AllOrError,
        count,
        _p: PhantomData,
    }
}

pub fn try_pop_bottom_many<T>(count: usize) -> PopBottomMany<T> {
    PopBottomMany {
        strictness: Strictness::AllOrError,
        count,
        _p: PhantomData,
    }
}

pub fn push_bottom_many<T>(elements: Vec<T>) -> PushBottomMany<T> {
    PushBottomMany { elements }
}

pub fn push_top_many<T>(elements: Vec<T>) -> PushTopMany<T> {
    PushTopMany { elements }
}

pub fn remove<T>(index: usize) -> Remove<T> {
    Remove {
        index,
        _p: PhantomData,
    }
}

pub fn clear<T>() -> Clear<T> {
    Clear { _p: PhantomData }
}

#[telety(crate::pile)]
#[tagset(Create<T>)]
#[tagset(Destroy<T>)]
#[tagset(Update<T>)]
#[tagset(PushTop<T>)]
#[tagset(PushBottom<T>)]
#[tagset(PopTop<T>)]
#[tagset(PopBottom<T>)]
#[tagset(PushTopMany<T>)]
#[tagset(PushBottomMany<T>)]
#[tagset(PopTopMany<T>)]
#[tagset(PopBottomMany<T>)]
#[tagset(Insert<T>)]
#[tagset(Remove<T>)]
#[tagset(Shuffle<T>)]
#[tagset(Clear<T>)]
#[tagset(reserved(..32))]
pub struct Actions<T>;

#[derive(Debug, Clone, spru::FromInfallible)]
#[amass_telety(crate::pile)]
pub enum Error {
    Pop(error::Pop),
    Insert(error::Insert),
    Remove(error::Remove),
}

pub type Create<T> = verbatim::Create<State<T>>;

pub type Destroy<T> = verbatim::Destroy<State<T>>;

pub type Update<T> = verbatim::Update<State<T>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct PushTop<T> {
    item: T,
}

impl<T> spru::action::Update for PushTop<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopTop<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        value.items.push_front(self.item.clone());
        Ok(pop_top())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, spru::action::Update)]
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

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        Ok(match value.items.pop_front() {
            Some(item) => Some(push_top(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(error::Pop::Empty)?,
            },
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct PushBottom<T> {
    item: T,
}

impl<T> spru::action::Update for PushBottom<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopBottom<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        value.items.push_back(self.item.clone());
        Ok(pop_bottom())
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

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        Ok(match value.items.pop_back() {
            Some(item) => Some(push_bottom(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(error::Pop::Empty)?,
            },
        })
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct PopTopMany<T> {
    strictness: Strictness,
    count: usize,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for PopTopMany<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PushTopMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self {
            strictness,
            count,
            _p,
        } = *self;

        let mut elements = vec![];

        if strictness == Strictness::AllOrError && value.items.len() <= count {
            Err(error::Pop::Empty.into())
        } else {
            for _ in 0..count {
                if let Some(element) = value.items.pop_front() {
                    elements.push(element);
                }
            }

            Ok(PushTopMany { elements })
        }
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct PopBottomMany<T> {
    strictness: Strictness,
    count: usize,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for PopBottomMany<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PushBottomMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self {
            strictness,
            count,
            _p,
        } = *self;

        let mut elements = vec![];

        if strictness == Strictness::AllOrError && value.items.len() <= count {
            Err(error::Pop::Empty.into())
        } else {
            for _ in 0..count {
                if let Some(element) = value.items.pop_front() {
                    elements.push(element);
                }
            }

            elements.reverse();

            Ok(PushBottomMany { elements })
        }
    }
}

#[derive_where(Default)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct PushTopMany<T> {
    elements: Vec<T>,
}

impl<T> spru::action::Update for PushTopMany<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopTopMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self { ref elements } = *self;

        for element in elements {
            value.items.push_front(element.clone());
        }

        Ok(PopTopMany {
            strictness: Strictness::AllOrError,
            count: elements.len(),
            _p: PhantomData,
        })
    }
}

impl<T> ops::Deref for PushTopMany<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

#[derive_where(Default)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct PushBottomMany<T> {
    elements: Vec<T>,
}

impl<T> spru::action::Update for PushBottomMany<T>
where
    T: Clone + Serial,
{
    type T = State<T>;
    type Undo = PopBottomMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self { ref elements } = *self;

        for element in elements {
            value.items.push_front(element.clone());
        }

        Ok(PopBottomMany {
            strictness: Strictness::AllOrError,
            count: elements.len(),
            _p: PhantomData,
        })
    }
}

impl<T> ops::Deref for PushBottomMany<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.elements
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

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        use rand::{Rng, SeedableRng};

        let mut rng = crate::Rng::seed_from_u64(self.seed);

        // Based on the soon-to-be-replaced rand 0.85 implementation
        fn gen_index(rng: &mut crate::Rng, ubound: usize) -> usize {
            if ubound <= (u32::MAX as usize) {
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
            for (i, index) in (1..value.items.len()).zip(deferred.into_iter().rev()) {
                value.items.swap(i, index);
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct Insert<T> {
    index: usize,
    element: T,
}

impl<T: Clone> spru::action::Update for Insert<T> {
    type T = State<T>;
    type Undo = Remove<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.index;
        if index <= value.items.len() {
            value.items.insert(index, self.element.clone());
            Ok(Remove {
                index,
                _p: PhantomData,
            })
        } else {
            Err(error::Insert::Index(index, value.items.len()).into())
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

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.index;
        if index < value.items.len() {
            let element = value.items.remove(index);
            Ok(Insert { index, element })
        } else {
            Err(error::Remove::Index(index, value.items.len()).into())
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

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let items = mem::take(&mut value.items);
        Ok(update(items.0))
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
        let (mut pile, _) = create.create().unwrap();

        shuffle(&mut rng()).update(&mut pile).unwrap();

        let orig_order = pile.items.clone();

        let undo = shuffle(&mut rng()).update(&mut pile).unwrap();

        assert_ne!(pile.items, orig_order);

        undo.update(&mut pile).unwrap();

        assert_eq!(pile.items, orig_order);
    }
}
