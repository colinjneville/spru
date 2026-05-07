pub mod error;

use std::{marker::PhantomData, mem, ops};

use derive_where::derive_where;
use spru::common::error::AnyResult;
use spru_script::script;
use tagset::tagset;
use telety::telety;

use crate::{Strictness, cloned, fail, maybe};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[script(include = [Methods])]
pub struct Pile<T> {
    /// front/top -> back/bottom
    items: FakeDeVec<T>,
}

#[script(partial = Methods)]
impl<T: Clone + 'static> Pile<T> {
    #[create]
    fn default() -> Create<T> {
        create(vec![])
    }

    #[create]
    fn new(items: Vec<T>) -> Create<T> {
        create(items)
    }

    #[method]
    fn destroy(&self) -> ((), Destroy<T>) {
        ((), destroy())
    }

    #[get(name = top)]
    fn _top(&self) -> Option<T> {
        self.top().cloned()
    }

    #[get(name = bottom)]
    fn _bottom(&self) -> Option<T> {
        self.bottom().cloned()
    }

    #[get]
    fn items(&self) -> Vec<T> {
        self.items.0.clone()
    }

    #[set(name = items)]
    fn items_set(&self, items: Vec<T>) -> (Update<T>, ) {
        (update(items), )
    }

    #[method]
    fn get(&self, index: usize) -> (Option<T>, maybe::Update<fail::Update<Pile<T>>>) {
        if let Some(value) = self.items.get(index) {
            (Some(value.clone()), maybe::no())
        } else {
            (None, maybe::yes(fail::fail(format!("Index {index} is out of range for pile with length {}", self.items.0.len()))))
        }
    }

    #[method]
    fn set(&self, index: usize, value: T) -> (T, Set<T>) {
        let prev = self.items.0.get(index)
            .cloned()
            // If we error, this value should never make it 
            // to the scripting environment, so it can be any valid T
            .unwrap_or_else(|| value.clone());
        (prev, set(index, value))
    }

    #[method]
    fn try_get(&self, index: usize) -> (Option<T>, ) {
        (self.items.get(index).cloned(), )
    }

    #[method(name = insert)]
    fn _insert(&self, index: usize, element: T) -> ((), Insert<T>) {
        ((), insert(index, element))
    }

    #[method(name = remove)]
    fn _remove(&self, index: usize) -> (Option<T>, Remove<T>) {
        (self.items.0.get(index).cloned(), remove(index))
    }

    #[method]
    fn push_top(&self, item: T) -> ((), PushTop<T>) {
        ((), push_top(item))
    }

    #[method]
    fn push_top_many(&self, items: Vec<T>) -> ((), PushTopMany<T>) {
        ((), push_top_many(items))
    }

    #[method]
    fn push_bottom(&self, item: T) -> ((), PushBottom<T>) {
        ((), push_bottom(item))
    }

    #[method]
    fn push_bottom_many(&self, items: Vec<T>) -> ((), PushBottomMany<T>) {
        ((), push_bottom_many(items))
    }

    #[method]
    fn pop_top(&self) -> (Option<T>, PopTop<T>) {
        (self._top(), pop_top())
    }

    #[method]
    fn pop_top_many(&self, count: usize) -> (Vec<T>, PopTopMany<T>) {
        (self.items.0[0..count].to_vec(), pop_top_many(count))
    }

    #[method]
    fn pop_bottom(&self) -> (Option<T>, PopBottom<T>) {
        (self._bottom(), pop_bottom())
    }

    #[method]
    fn pop_bottom_many(&self, count: usize) -> (Vec<T>, PopBottomMany<T>) {
        let start = self.items.0.len().saturating_sub(count);
        let mut v = self.items.0[start..].to_vec();
        v.reverse();
        (v, pop_bottom_many(count))
    }

    #[method]
    fn try_pop_top(&self) -> (Option<T>, PopTop<T>) {
        (self._top(), try_pop_top())
    }

    #[method]
    fn try_pop_top_many(&self, count: usize) -> (Vec<T>, PopTopMany<T>) {
        (self.items.0.get(0..count).unwrap_or(&[]).to_vec(), try_pop_top_many(count))
    }

    #[method]
    fn try_pop_bottom(&self) -> (Option<T>, PopBottom<T>) {
        (self._bottom(), try_pop_bottom())
    }

    #[method]
    fn try_pop_bottom_many(&self, count: usize) -> (Vec<T>, PopBottomMany<T>) {
        let start = self.items.0.len().saturating_sub(count);
        (self.items.0.get(start..).unwrap_or(&[]).to_vec(), try_pop_bottom_many(count))
    }

    #[method]
    fn clear(&self) -> ((), Clear<T>) {
        ((), clear())
    }

    #[method]
    fn shuffle(&self) -> ((), Shuffle<T>) {
        let mut rng = rand::rng();
        ((), shuffle(&mut rng))
    }
}

impl<T> Pile<T> {
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

impl<T> ops::Deref for Pile<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<'i, T> IntoIterator for &'i Pile<T> {
    type Item = &'i T;
    type IntoIter = std::slice::Iter<'i, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
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
#[tagset(PushTopMany<T>)]
#[tagset(PushBottomMany<T>)]
#[tagset(PopTopMany<T>)]
#[tagset(PopBottomMany<T>)]
#[tagset(Set<T>)]
#[tagset(Insert<T>)]
#[tagset(Remove<T>)]
#[tagset(Shuffle<T>)]
#[tagset(Clear<T>)]
#[tagset(reserved(..32))]
pub struct Actions<T>;

pub fn create<T>(items: impl IntoIterator<Item = T>) -> Create<T> {
    cloned::create(Pile {
        items: items.into_iter().collect(),
    })
}

pub fn default<T>() -> Create<T> {
    create([])
}

pub fn destroy<T>() -> Destroy<T> {
    cloned::destroy()
}

pub fn update<T>(items: impl IntoIterator<Item = T>) -> Update<T> {
    cloned::update(Pile {
        items: items.into_iter().collect(),
    })
}

pub fn set<T>(index: usize, element: T) -> Set<T> {
    Set {
        index,
        element,
    }
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
        strictness: Strictness::BestEffort,
        count,
        _p: PhantomData,
    }
}

pub fn pop_bottom_many<T>(count: usize) -> PopBottomMany<T> {
    PopBottomMany {
        strictness: Strictness::AllOrError,
        count,
        _p: PhantomData,
    }
}

pub fn try_pop_bottom_many<T>(count: usize) -> PopBottomMany<T> {
    PopBottomMany {
        strictness: Strictness::BestEffort,
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

pub fn insert<T>(index: usize, element: T) -> Insert<T> {
    Insert {
        index,
        element,
    }
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

// An inefficient temporary stand-in for a single-slice deque
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive_where(Default; )]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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

pub type Create<T> = cloned::Create<Pile<T>>;

pub type Destroy<T> = cloned::Destroy<Pile<T>>;

pub type Update<T> = cloned::Update<Pile<T>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct PushTop<T> {
    item: T,
}

impl<T> spru::action::Update for PushTop<T>
where
    T: Clone,
{
    type T = Pile<T>;
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
    T: Clone,
{
    type T = Pile<T>;
    type Undo = PushTop<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        Ok(match value.items.pop_front() {
            Some(item) => Some(push_top(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(error::Empty)?,
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
    T: Clone,
{
    type T = Pile<T>;
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
    T: Clone,
{
    type T = Pile<T>;
    type Undo = PushBottom<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        Ok(match value.items.pop_back() {
            Some(item) => Some(push_bottom(item)),
            None => match self.strictness {
                Strictness::BestEffort => None,
                Strictness::AllOrError => Err(error::Empty)?,
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
    T: Clone,
{
    type T = Pile<T>;
    type Undo = PushTopMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self {
            strictness,
            count,
            _p,
        } = *self;

        let mut elements = vec![];

        if strictness == Strictness::AllOrError && value.items.len() <= count {
            Err(error::Empty.into())
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
    T: Clone,
{
    type T = Pile<T>;
    type Undo = PushBottomMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self {
            strictness,
            count,
            _p,
        } = *self;

        let mut elements = vec![];

        if strictness == Strictness::AllOrError && value.items.len() <= count {
            Err(error::Empty.into())
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
    T: Clone,
{
    type T = Pile<T>;
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
    T: Clone,
{
    type T = Pile<T>;
    type Undo = PopBottomMany<T>;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let Self { ref elements } = *self;

        for element in elements.iter().rev() {
            value.items.push_back(element.clone());
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
    T: Clone,
{
    type T = Pile<T>;
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
pub struct Set<T> {
    index: usize,
    element: T,
}

impl<T: Clone> spru::action::Update for Set<T> {
    type T = Pile<T>;
    type Undo = Set<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let Self {
            index,
            ref element,
        } = *self;

        if let Some(existing) = value.items.0.get_mut(self.index) {
            let element = std::mem::replace(existing, element.clone());
            Ok(Set {
                index,
                element,
            })
        } else {
            Err(error::IndexOutOfRange { index, len: value.items.len() }.into())
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, spru::action::Update)]
pub struct Insert<T> {
    index: usize,
    element: T,
}

impl<T: Clone> spru::action::Update for Insert<T> {
    type T = Pile<T>;
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
            Err(error::IndexOutOfRange { index, len: value.items.len() }.into())
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
    type T = Pile<T>;
    type Undo = Insert<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let index = self.index;
        if index < value.items.len() {
            let element = value.items.remove(index);
            Ok(Insert { index, element })
        } else {
            Err(error::IndexOutOfRange { index, len: value.items.len() }.into())
        }
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Update)]
pub struct Clear<T> {
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Clear<T> {
    type T = Pile<T>;
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

    use crate::pile::Pile;

    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(impl spru::State)]
    #[tagset(impl<Action, Registry> spru_script::ScriptableState<Action, Registry>)]
    #[tagset(derive(Debug))]
    #[tagset(Pile<i32>)]
    struct MyState;

    #[tagset(impl spru::Action {
        type State = MyState;
    })]
    #[tagset(impl tagset::proxy::serde::Serialize)]
    #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
    #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
    #[tagset(derive(Debug, Clone))]
    #[tagset(include(super::Actions<i32>))]
    #[tagset(crate::maybe::Update<crate::fail::Update<Pile<i32>>>)]
    #[tagset(crate::fail::Update<Pile<i32>>)]
    struct MyAction;

    #[test]
    fn test_script() {
        use spru_script::Language as _;

        let storage = crate::storage::Standalone::<MyState>::new();

        let lua = spru_script_lua::Lua::<MyState, MyAction>::new();

        let mut test_interactor = spru::interactor::test_util::TestInteractor::new(storage);

        let script = r#"
            local output = {}
            output.insert = table.insert

            local deck = Pile[i32].default()

            -- (/)
            output:insert(deck:try_pop_bottom())
            -- (/)
            output:insert(deck:try_pop_top())
            -- (/)
            for i, value in ipairs(deck:try_pop_bottom_many(5)) do
                output:insert(value)
            end
            -- (/)
            for i, value in ipairs(deck:try_pop_top_many(5)) do
                output:insert(value)
            end
            
            deck:push_top(6)
            deck:push_top(7)
            deck:push_top_many({8, 9, 10})
            deck:push_bottom(5)
            deck:push_bottom(4)
            deck:push_bottom_many({1, 2, 3})

            -- 10...1
            for i, value in ipairs(deck.items) do
                output:insert(value)
            end

            -- 10
            output:insert(deck:pop_top())
            -- 9
            output:insert(deck.top)
            -- 1
            output:insert(deck:pop_bottom())
            -- 2
            output:insert(deck.bottom)
            -- 9, 8
            for i, value in ipairs(deck:pop_top_many(2)) do
                output:insert(value)
            end
            -- 2, 3
            for i, value in ipairs(deck:pop_bottom_many(2)) do
                output:insert(value)
            end

            -- 11
            deck:set(2, 11)
            output:insert(deck:get(2))

            -- 21
            deck.items = {20, 21, 22}
            output:insert(deck:get(1))

            -- 21
            deck:insert(1, 31)
            output:insert(deck:get(2))

            -- 31
            deck:remove(0)
            output:insert(deck:get(0))

            deck:clear()

            -- (/)
            output:insert(deck:try_pop_top())

            return output
        "#;

        let mut interactor = test_interactor.interactor::<MyAction, _>(7);
        let value: Vec<i32> = lua.exec(&mut interactor, script).unwrap();

        assert_eq!(
            value, 
            vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 10, 9, 1, 2, 9, 8, 2, 3, 11, 21, 21, 31, ],
        );
    }
}
