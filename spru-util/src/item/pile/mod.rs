pub mod error;
pub mod update;

use std::{collections::VecDeque, marker::PhantomData};

use amass::amass_telety;
use perfect_derive::perfect_derive;
use spru::Serial;
use update::{PopBottom, PopTop, PushBottom, PushTop, Shuffle};

#[perfect_derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Pile<T> {
    /// front/top -> back/bottom
    items: VecDeque<T>,
}

impl<T> Pile<T> {
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

impl<'i, T> IntoIterator for &'i Pile<T> {
    type Item = &'i T;
    type IntoIter = std::collections::vec_deque::Iter<'i, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Catalog)]
#[catalog(error = Error)]
#[amass_telety(crate::item::pile)]
pub enum Catalog<T> {
    Create(Create<T>),
    PushTop(PushTop<T>),
    PushBottom(PushBottom<T>),
    PopTop(PopTop<T>),
    PopBottom(PopBottom<T>),
    Shuffle(Shuffle<T>),
    Destroy(Destroy<T>),
}

#[perfect_derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[amass_telety(crate::item::pile)]
pub enum Error {
    Pop(error::Pop),
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::create(Undo = Destroy<T>)]
pub struct Create<T> {
    items: Vec<T>,
}

impl<T> Create<T> {
    pub fn new(items: impl IntoIterator<Item=T>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }
}

impl<T> spru::Action for Create<T>
where
    T: spru::Serial + Clone,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, _input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok((Destroy::default(), Pile { items: self.items.iter().cloned().collect() }))
    }
}

#[perfect_derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::destroy(Undo = Create<T>)]
pub struct Destroy<T>(PhantomData<fn() -> T>);

impl<T> Destroy<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> spru::Action for Destroy<T>
where
    T: Clone + Serial,
{
    type T = Pile<T>;
    
    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(Create { items: input.items.into() })
    }
}

#[cfg(test)]
mod test {
    use rand::{thread_rng, Rng};
    use spru::item::Mut;

    use crate::item::pile::update::Shuffle;

    use super::*;

    #[test]
    fn shuffle() {
        type Lookup = crate::lookup::Standalone;

        let create = Create::new([1, 2, 3, 4, 5, 6, 7, 8]);
        let pile = spru::Action::apply::<Lookup>(&create, ()).unwrap().into().out;
        let mut pile = spru::Item::test_zero(pile);

        spru::Action::apply::<Lookup>(&Shuffle::new(thread_rng().gen()), Mut::test_new(&mut pile)).unwrap();

        let orig_order = pile.items.clone();

        let seed = thread_rng().gen();

        let undo = spru::Action::apply::<Lookup>(&Shuffle::new(seed), Mut::test_new(&mut pile)).unwrap().into().undo;

        assert_ne!(pile.items, orig_order);

        spru::Action::apply::<Lookup>(&undo.unwrap(), Mut::test_new(&mut pile)).unwrap();

        assert_eq!(pile.items, orig_order);
    }
}