use std::{marker::PhantomData, mem};

use perfect_derive::perfect_derive;
use spru::{action, Serial};

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Catalog)]
#[amass::amass_telety(crate::action::verbatim)]
pub enum Catalog<T> {
    Create(Create<T>),
    Update(Update<T>),
    Destroy(Destroy<T>),
}

#[perfect_derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::create(Undo = Destroy<T>)]
pub struct Create<T> {
    value: T,
}

impl<T> spru::Action for Create<T> 
where
    T: Clone + ::spru::Serial,
{
    type T = T;

    fn apply<'l, Lookup>(&self, _input: action::In<'l, Self, Lookup>) -> Result<impl Into<action::Output<Self::Undo, action::Out<Self, Lookup>>>, Self::Error> 
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l,
    {
        Ok((Destroy::default(), self.value.clone()))
    }    
}

impl<T> Create<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
        }
    }
}

impl<T> From<T> for Create<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::destroy(Undo = Create<T>)]
pub struct Destroy<T>(PhantomData<fn() -> T>);

impl<T> spru::Action for Destroy<T>
where
    T: Clone + spru::Serial,
{
    type T = T;

    fn apply<'l, Lookup>(&self, input: action::In<'l, Self, Lookup>) -> Result<impl Into<action::Output<Self::Undo, action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(Create::new(input))
    }
}

impl<T> Destroy<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for Destroy<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[perfect_derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update]
pub struct Update<T> {
    value: T,
}

impl<T> spru::Action for Update<T>
where 
    T: Clone + Serial,
{
    type T = T;

    fn apply<'l, Lookup>(&self, mut input: action::In<'l, Self, Lookup>) -> Result<impl Into<action::Output<Self::Undo, action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        let old = mem::replace(&mut *input, self.value.clone());
        Ok(Self::new(old))
    }
}

impl<T> Update<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
        }
    }
}

impl<T> From<T> for Update<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
