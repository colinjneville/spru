mod range_type;
pub use range_type::RangeType;

use std::{ops::{self, RangeBounds}, cmp};

use derive_where::derive_where;
use num_traits::Signed;
use tagset::tagset;
use telety::telety;

use crate::{verbatim, AddSigned, Strictness};

/// Types able to be used as a counter value
pub trait CounterType: cmp::PartialOrd + ops::Sub<Output=Self> + num_traits::One + Clone { }

impl<T> CounterType for T
where T: cmp::PartialOrd + ops::Sub<Output=Self> + num_traits::One + Clone { }

#[derive(Debug, Default, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct State<T> {
    value: T,
    bounds: RangeType<T>,
}

impl<T: AddSigned> State<T> {
    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn bounds(&self) -> &RangeType<T> {
        &self.bounds
    }
}

pub fn create<T>(value: T) -> Create<T> {
    create_bounded(
        value,
        RangeType::default(),
    )
}

pub fn create_bounded<T>(value: T, bounds: impl Into<RangeType<T>>) -> Create<T> {
    let bounds = bounds.into();

    verbatim::create(State {
        value,
        bounds,
    })
}

pub fn default<T: Default>() -> Create<T> {
    create(T::default())
}

pub fn add<T: AddSigned>(value: T::Signed, strictness: Strictness) -> Add<T> {
    Add {
        value,
        strictness,
    }
}

pub fn add_checked<T: AddSigned>(value: T::Signed) -> Add<T> {
    add(value, Strictness::AllOrError)
}

pub fn add_saturating<T: AddSigned>(value: T::Signed) -> Add<T> {
    add(value, Strictness::BestEffort)
}

pub fn destroy<T>() -> Destroy<T> {
    verbatim::destroy()
}

#[telety(crate::counter)]
#[tagset(verbatim::Create<State<T>>)]
#[tagset(Add<T>)]
#[tagset(verbatim::Destroy<State<T>>)]
#[tagset(reserved(..8))]
pub struct Actions<T: AddSigned>;

pub type Create<T> = verbatim::Create<State<T>>;

#[derive_where(Debug, Clone; T::Signed)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Add<T: AddSigned> {
    #[serde(bound(serialize = "T::Signed: serde::Serialize", deserialize = "T::Signed: serde::Deserialize<'de>"))]
    value: T::Signed, 
    strictness: Strictness, 
}

impl<T> spru::action::Update for Add<T>
where 
    T: CounterType + Ord + AddSigned<Signed: spru::Serial> + spru::Serial,
{
    type T = State<T>;
    type Undo = Self;
    type Error = Error<T>;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let sum = match self.strictness {
            Strictness::BestEffort => value.bounds.constrain(value.value.saturating_add(self.value)),
            Strictness::AllOrError => value.value.checked_add(self.value)
                .filter(|v| value.bounds.contains(v))
                .ok_or(Error::InvalidModifier { value: value.value, modifier: self.value })?,
        };

        // Absolute value of diff (T)
        let diff = sum.max(value.value) - sum.min(value.value);

        // Convert to U and correct sign
        let diff = diff.into_signed() * -self.value.signum();

        value.value = sum;
        
        Ok(Some(Self {
            value: diff,
            strictness: Strictness::AllOrError,
        }))
    }
}

impl<T> spru::action::UpdateReturn for Add<T>
where Self: spru::action::Update
{
    type ReturnValue = T;

    fn return_value(&self, t: &Self::T) -> Self::ReturnValue {
        todo!()
    }
}

pub type Destroy<T> = verbatim::Destroy<State<T>>;

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[derive(thiserror::Error)]
pub enum Error<T: AddSigned> {
    #[error("Could not modify value of {value} by {modifier}")]
    InvalidModifier { value: T, modifier: T::Signed }
}

#[cfg(test)]
mod test {
    use spru::action::{Create, Update as _};

    use super::*;

    #[test]
    fn update_unbounded() {
        let (mut counter, _) = create(3u32).create().unwrap();

        add_checked(-5i32).update(&mut counter)
            .unwrap_err();

        let undo = add_checked(-2).update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &1);

        undo.unwrap().update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &3);

        let undo = add_saturating(-5).update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &0);

        undo.unwrap().update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &3);
    }

    #[test]
    fn update_bounded() {
        let (mut counter, _) = create_bounded(3u32, 2..).create().unwrap();

        let undo = add_saturating(-5).update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &2);

        undo.unwrap().update(&mut counter)
            .unwrap();

        assert_eq!(counter.value(), &3);
    }
}