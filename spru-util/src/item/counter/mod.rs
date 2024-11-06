mod range_type;
pub use range_type::RangeType;

use std::{marker::PhantomData, ops::{self, RangeBounds}, cmp};

use amass::amass_telety;
use perfect_derive::perfect_derive;
use num_traits::Signed;

use crate::{AddSigned, Strictness};

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Catalog)]
#[catalog(error = Error::<T>)]
#[amass_telety(crate::item::counter)]
pub enum Catalog<T: AddSigned> {
    Create(Create<T>),
    #[serde(bound(serialize = "T::Signed: serde::Serialize", deserialize = "T::Signed: serde::Deserialize<'de>"))]
    Add(Add<T>),
    Destroy(Destroy<T>),
}

pub trait CounterType: cmp::PartialOrd + ops::Sub<Output=Self> + num_traits::One + Clone { }

impl<T> CounterType for T
where T: cmp::PartialOrd + ops::Sub<Output=Self> + num_traits::One + Clone { }


#[derive(Debug, Default, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Counter<T> {
    value: T,
    bounds: RangeType<T>,
}

impl<T> Counter<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            bounds: Default::default(),
        }
    }

    fn new_bounded(value: T, bounds: impl Into<RangeType<T>>) -> Self {
        let bounds = bounds.into();
        Self {
            value,
            bounds,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn bounds(&self) -> &RangeType<T> {
        &self.bounds
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::create(Undo = Destroy<T>)]
pub struct Create<T> {
    value: T, 
    bounds: RangeType<T>,
}

impl<T> Create<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            bounds: RangeType::default(),
        }
    }

    pub fn new_bounded(value: T, bounds: impl Into<RangeType<T>>) -> Self {
        let bounds = bounds.into();
        Self {
            value,
            bounds,
        }
    }
}

impl<T> spru::Action for Create<T> 
where
    T: CounterType + spru::Serial,
{
    type T = Counter<T>;
    
    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok((Destroy::default(), Counter { value: self.value.clone(), bounds: self.bounds.clone(), }))
    }
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Error = Error<T>)]
pub struct Add<T: AddSigned> {
    #[serde(bound(serialize = "T::Signed: serde::Serialize", deserialize = "T::Signed: serde::Deserialize<'de>"))]
    value: T::Signed, 
    strictness: Strictness, 
}

impl<T: AddSigned> Add<T> {
    pub fn new(value: T::Signed, strictness: Strictness) -> Self {
        Self {
            value,
            strictness,
        }
    }

    pub fn new_checked(value: T::Signed) -> Self {
        Self::new(value, Strictness::AllOrError)
    }

    pub fn new_saturating(value: T::Signed) -> Self {
        Self::new(value, Strictness::BestEffort)
    }
}

impl<T> spru::Action for Add<T> 
where 
    T: CounterType + Ord + AddSigned<Signed: spru::Serial> + spru::Serial,
{    
    type T = Counter<T>;
    
    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        let sum = match self.strictness {
            Strictness::BestEffort => input.bounds.constrain(input.value.saturating_add(self.value)),
            Strictness::AllOrError => input.value.checked_add(self.value).filter(|v| input.bounds.contains(v)).ok_or(Error::InvalidModifier { value: input.value, modifier: self.value })?,
        };

        // Absolute value of diff (T)
        let diff = sum.max(input.value) - sum.min(input.value);

        // Convert to U and correct sign
        let diff = diff.into_signed() * -self.value.signum();

        input.value = sum;
        
        Ok(Self {
            value: diff,
            strictness: Strictness::AllOrError,
        })
    }

    
}

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::destroy(Undo = Create<T>)]
pub struct Destroy<T>(PhantomData<fn() -> T>);

impl<T> spru::Action for Destroy<T>
where 
    T: 'static,
{
    type T = Counter<T>;
    
    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l {
        Ok(Create { 
            value: input.value,
            bounds: input.bounds,
        })
    }
}

impl<T> Default for Destroy<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[derive(thiserror::Error)]
pub enum Error<T: AddSigned> {
    #[error("Could not modify value of {value} by {modifier}")]
    InvalidModifier { value: T, modifier: T::Signed }
}

#[cfg(test)]
mod test {
    use spru::item::Mut;

    use super::*;

    #[test]
    fn update() {
        type Lookup = crate::lookup::Standalone;

        let mut counter = spru::Item::test_zero(Counter::<u32>::new(3));

        spru::Action::apply::<Lookup>(&Add::new_checked(-5i32), Mut::test_new(&mut counter)).err().unwrap();

        let spru::action::Output { undo, .. } = spru::Action::apply::<Lookup>(&Add::new_checked(-2), Mut::test_new(&mut counter)).unwrap().into();

        assert_eq!(counter.value(), &1);

        spru::Action::apply::<Lookup>(&undo.unwrap(), Mut::test_new(&mut counter)).unwrap();

        assert_eq!(counter.value(), &3);

        let undo = spru::Action::apply::<Lookup>(&Add::new_saturating(-5), Mut::test_new(&mut counter)).unwrap().into().undo;

        assert_eq!(counter.value(), &0);

        spru::Action::apply::<Lookup>(&undo.unwrap(), Mut::test_new(&mut counter)).unwrap();

        assert_eq!(counter.value(), &3);

        let mut counter = spru::Item::test_zero(Counter::<u32>::new_bounded(3, 2..));

        let undo = spru::Action::apply::<Lookup>(&Add::new_saturating(-5), Mut::test_new(&mut counter)).unwrap().into().undo;

        assert_eq!(counter.value(), &2);

        spru::Action::apply::<Lookup>(&undo.unwrap(), Mut::test_new(&mut counter)).unwrap();

        assert_eq!(counter.value(), &3);
    }
}