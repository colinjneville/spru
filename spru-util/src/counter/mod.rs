pub mod error;
mod range_type;
pub use range_type::RangeType;
use spru::common::error::AnyResult;
use spru_script::script;

use std::{
    cmp, fmt, ops::{self, RangeBounds}
};

use derive_where::derive_where;
use num_traits::Signed;
use tagset::tagset;
use telety::telety;

use crate::{Strictness, bounds, cloned};

/// Types able to be used as a counter value
pub trait CounterType: cmp::PartialOrd + ops::Sub<Output = Self> + num_traits::One + Clone {}

impl<T> CounterType for T where
    T: cmp::PartialOrd + ops::Sub<Output = Self> + num_traits::One + Clone
{
}

/// A numerical counter, for points, life totals, round numbers, etc.
/// You can also apply minimum and/or maximum bounds.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
#[script(include = [Methods])]
pub struct Counter<T> {
    #[get]
    value: T,
    bounds: RangeType<T>,
}

#[script(partial = Methods)]
impl<T> Counter<T>
where 
    T: CounterType + Ord + bounds::AddSigned + Default + 'static,
{
    #[create]
    fn create(value: T) -> cloned::Create<Counter<T>> {
        create(value)
    }

    // TODO how should bounds be handled in script?

    #[create]
    fn dflt() -> cloned::Create<Counter<T>> {
        create(T::default())
    }

    #[method]
    fn destroy(&self) -> ((), cloned::Destroy<Counter<T>>) {
        ((), destroy())
    }

    #[set(name = value)]
    fn value_set(&self, value: T) -> (Set<T>, ) {
        (set(value), )
    }

    #[method]
    fn set_clamped(&self, value: T) -> (T, Set<T>) {
        (self.bounds.constrain(value), set_clamped(value))
    }

    #[method]
    fn add_saturating(&self, value: T::Signed) -> (T, Add<T>) {
        let add = add_saturating(value);
        let sum = add.sum(self)
            .ok()
            .expect("Saturating add cannot fail");
        (sum, add)
    }

    #[method]
    fn add_checked(&self, value: T::Signed) -> (T, Add<T>) {
        let add = add_checked(value);
        let sum = add.sum(self)
            .ok()
            .unwrap_or(self.value);
        (sum, add)
    }
}

impl<T: bounds::AddSigned> Counter<T> {
    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn bounds(&self) -> &RangeType<T> {
        &self.bounds
    }
}

pub fn create<T>(value: T) -> Create<T> {
    create_bounded(value, RangeType::default())
}

pub fn create_bounded<T>(value: T, bounds: impl Into<RangeType<T>>) -> Create<T> {
    let bounds = bounds.into();

    cloned::create(Counter { value, bounds })
}

pub fn default<T: Default>() -> Create<T> {
    create(T::default())
}

pub fn set<T: bounds::AddSigned>(value: T) -> Set<T> {
    Set {
        value,
        strictness: Strictness::AllOrError,
    }
}

pub fn set_clamped<T: bounds::AddSigned>(value: T) -> Set<T> {
    Set {
        value,
        strictness: Strictness::BestEffort,
    }
}

pub fn add<T: bounds::AddSigned>(value: T::Signed, strictness: Strictness) -> Add<T> {
    Add { value, strictness }
}

pub fn add_checked<T: bounds::AddSigned>(value: T::Signed) -> Add<T> {
    add(value, Strictness::AllOrError)
}

pub fn add_saturating<T: bounds::AddSigned>(value: T::Signed) -> Add<T> {
    add(value, Strictness::BestEffort)
}

pub fn destroy<T>() -> Destroy<T> {
    cloned::destroy()
}

#[telety(crate::counter)]
#[tagset(cloned::Create<Counter<T>>)]
#[tagset(Add<T>)]
#[tagset(Set<T>)]
#[tagset(cloned::Destroy<Counter<T>>)]
#[tagset(reserved(..8))]
pub struct Actions<T: bounds::AddSigned>;

pub type Create<T> = cloned::Create<Counter<T>>;

#[derive_where(Debug, Clone; T::Signed)]
#[derive(serde::Serialize, serde::Deserialize, spru::action::Update)]
#[must_use]
pub struct Set<T: bounds::AddSigned> {
    value: T,
    strictness: Strictness,
}

impl<T: bounds::AddSigned + CounterType + Ord + fmt::Display + 'static> spru::action::Update for Set<T> {
    type T = Counter<T>;
    type Undo = Self;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let prev = value.value.clone();
        let bounded = value.bounds.constrain(self.value);
        if self.strictness == Strictness::AllOrError && self.value == bounded {
            Err(error::ValueOutOfBounds::new(self.value.clone(), value.bounds.clone()).into())
        } else {
            value.value = bounded;
            Ok(Self {
                value: prev,
                strictness: Strictness::AllOrError,
            })
        }
    }
}

#[derive_where(Debug, Clone; T::Signed)]
#[derive(serde::Serialize, serde::Deserialize, spru::action::Update)]
#[must_use]
pub struct Add<T: bounds::AddSigned> {
    #[serde(bound(
        serialize = "T::Signed: serde::Serialize",
        deserialize = "T::Signed: serde::Deserialize<'de>"
    ))]
    value: T::Signed,
    strictness: Strictness,
}

impl<T> Add<T>
where
    T: CounterType + Ord + bounds::AddSigned,
{
    fn sum(&self, value: &Counter<T>) -> Result<T, self::Error<T>> {
        match self.strictness {
            Strictness::BestEffort => Ok(value
                .bounds
                .constrain(value.value.saturating_add(self.value))),
            Strictness::AllOrError => value
                .value
                .checked_add(self.value)
                .filter(|v| value.bounds.contains(v))
                .ok_or(Error::InvalidModifier {
                    value: value.value,
                    modifier: self.value,
                }),
        }
    }
}

impl<T> spru::action::Update for Add<T>
where
    T: CounterType + Ord + bounds::AddSigned + 'static,
{
    type T = Counter<T>;
    type Undo = Self;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        let sum = self.sum(value)?;

        // Absolute value of diff (T)
        let diff = sum.max(value.value) - sum.min(value.value);

        // Convert to U and correct sign
        let diff = diff.into_signed() * -self.value.signum();

        value.value = sum;

        Ok(Self {
            value: diff,
            strictness: Strictness::AllOrError,
        })
    }
}

pub type Destroy<T> = cloned::Destroy<Counter<T>>;

#[derive(Debug, Clone, crate::FromInfallible, thiserror::Error)]
pub enum Error<T: bounds::AddSigned> {
    #[error("Could not modify value of {value} by {modifier}")]
    InvalidModifier { value: T, modifier: T::Signed },
}

#[cfg(test)]
mod test {
    use spru::action::{Create, Update as _};

    use super::*;

    #[test]
    fn update_unbounded() {
        let (mut counter, _) = create(3u32).create().unwrap();

        add_checked(-5i32)
            .update(&mut counter)
            .map(Into::into)
            .unwrap_err();

        let undo = add_checked(-2)
            .update(&mut counter)
            .map(Into::into)
            .unwrap();

        assert_eq!(counter.value(), &1);

        undo.unwrap().update(&mut counter).map(Into::into).unwrap();

        assert_eq!(counter.value(), &3);

        let undo = add_saturating(-5)
            .update(&mut counter)
            .map(Into::into)
            .unwrap();

        assert_eq!(counter.value(), &0);

        undo.unwrap().update(&mut counter).map(Into::into).unwrap();

        assert_eq!(counter.value(), &3);
    }

    #[test]
    fn update_bounded() {
        let (mut counter, _) = create_bounded(3u32, 2..).create().unwrap();

        let undo = add_saturating(-5)
            .update(&mut counter)
            .map(Into::into)
            .unwrap();

        assert_eq!(counter.value(), &2);

        undo.unwrap().update(&mut counter).map(Into::into).unwrap();

        assert_eq!(counter.value(), &3);
    }
}
