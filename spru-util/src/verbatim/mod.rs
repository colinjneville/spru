use std::{marker::PhantomData, mem};

use derive_where::derive_where;
use spru::common::error::AnyResult;
use tagset::tagset;
use telety::telety;

#[telety(crate::verbatim)]
#[tagset(Create<T>)]
#[tagset(Update<T>)]
#[tagset(Destroy<T>)]
#[tagset(reserved(..8))]
pub struct Actions<T>;

pub fn create<T>(value: T) -> Create<T> {
    Create { value }
}

pub fn default<T: Default>() -> Create<T> {
    create(T::default())
}

pub fn update<T>(value: T) -> Update<T> {
    Update { value }
}

pub fn update_default<T: Default>() -> Update<T> {
    update(T::default())
}

pub fn destroy<T>() -> Destroy<T> {
    Destroy(PhantomData)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, spru::action::Create)]
#[must_use]
pub struct Create<T> {
    value: T,
}

impl<T> spru::action::Create for Create<T>
where
    T: Clone,
{
    type T = T;
    type Undo = Destroy<T>;

    fn create(&self) -> AnyResult<(Self::T, Self::Undo)> {
        Ok((self.value.clone(), Destroy(PhantomData)))
    }
}

#[derive_where(Debug, Clone, Serialize, Deserialize)]
#[derive(spru::action::Destroy)]
#[must_use]
pub struct Destroy<T>(PhantomData<fn() -> T>);

impl<T> spru::action::Destroy for Destroy<T> {
    type T = T;
    type Undo = Create<T>;

    fn destroy(&self, value: Self::T) -> AnyResult<Self::Undo> {
        Ok(Create { value })
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, spru::action::Update)]
#[must_use]
pub struct Update<T> {
    value: T,
}

impl<T: Clone> spru::action::Update for Update<T> {
    type T = T;
    type Undo = Self;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let old = mem::replace(&mut *value, self.value.clone());
        Ok(Self { value: old })
    }
}
