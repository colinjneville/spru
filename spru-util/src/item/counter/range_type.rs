use std::ops::{self, RangeBounds as _};

use perfect_derive::perfect_derive;

use super::CounterType;


#[perfect_derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RangeType<T> {
    Full(ops::RangeFull),
    From(ops::RangeFrom<T>),
    Inclusive(ops::RangeInclusive<T>),
    ToInclusive(ops::RangeToInclusive<T>),
    Exclusive(ops::Range<T>),
    ToExclusive(ops::RangeTo<T>),
}

impl<T: CounterType> RangeType<T> {
    pub fn constrain(&self, value: T) -> T {
        let start = match self.start_bound() {
            ops::Bound::Included(start) => Some(start.clone()),
            ops::Bound::Excluded(_) |
            ops::Bound::Unbounded => None,
        };
        let end = match self.end_bound() {
            ops::Bound::Included(end) => Some(end.clone()),
            ops::Bound::Excluded(end) => Some(end.clone() - num_traits::One::one()),
            ops::Bound::Unbounded => None,
        };
        match (start, end) {
            (None, None) => value,
            (None, Some(end)) => num_traits::clamp_max(value, end),
            (Some(start), None) => num_traits::clamp_min(value, start),
            (Some(start), Some(end)) => num_traits::clamp(value, start, end),
        }
    }
}

impl<T: serde::Serialize> serde::Serialize for RangeType<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let start = self.start_bound();
        let end = self.end_bound();
        (start, end).serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for RangeType<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let (start, end) = <(ops::Bound::<T>, ops::Bound::<T>)>::deserialize(deserializer)?;
        let value = match (start, end) {
            // Technically a fully-iterated `..=` has an `Excluded` end `Bound`,
            // but even if we are given such a range, we don't care about preserving
            // that property
            (ops::Bound::Included(start), ops::Bound::Included(end)) => Self::Inclusive(start..=end),
            (ops::Bound::Included(start), ops::Bound::Excluded(end)) => Self::Exclusive(start..end),
            (ops::Bound::Included(start), ops::Bound::Unbounded) => Self::From(start..),
            (ops::Bound::Unbounded, ops::Bound::Included(end)) => Self::ToInclusive(..=end),
            (ops::Bound::Unbounded, ops::Bound::Excluded(end)) => Self::ToExclusive(..end),
            (ops::Bound::Unbounded, ops::Bound::Unbounded) => Self::Full(ops::RangeFull),
            (ops::Bound::Excluded(_), _) => return Err(<D::Error as serde::de::Error>::invalid_value(serde::de::Unexpected::Other("excluded start bound"), &"included or unbounded start")),
        };
        Ok(value)
    }
}

impl<T> Default for RangeType<T> {
    fn default() -> Self {
        Self::Full(ops::RangeFull)
    }
}

impl<T> ops::RangeBounds<T> for RangeType<T> {
    fn start_bound(&self) -> ops::Bound<&T> {
        match self {
            RangeType::Full(r) => r.start_bound(),
            RangeType::From(r) => r.start_bound(),
            RangeType::Inclusive(r) => r.start_bound(),
            RangeType::ToInclusive(r) => r.start_bound(),
            RangeType::Exclusive(r) => r.start_bound(),
            RangeType::ToExclusive(r) => r.start_bound(),
        }
    }

    fn end_bound(&self) -> ops::Bound<&T> {
        match self {
            RangeType::Full(r) => r.end_bound(),
            RangeType::From(r) => r.end_bound(),
            RangeType::Inclusive(r) => r.end_bound(),
            RangeType::ToInclusive(r) => r.end_bound(),
            RangeType::Exclusive(r) => r.end_bound(),
            RangeType::ToExclusive(r) => r.end_bound(),
        }
    }
}

impl<T> From<ops::RangeFull> for RangeType<T> {
    fn from(value: ops::RangeFull) -> Self {
        Self::Full(value)
    }
}

impl<T> From<ops::RangeFrom<T>> for RangeType<T> {
    fn from(value: ops::RangeFrom<T>) -> Self {
        Self::From(value)
    }
}

impl<T> From<ops::RangeInclusive<T>> for RangeType<T> {
    fn from(value: ops::RangeInclusive<T>) -> Self {
        Self::Inclusive(value)
    }
}

impl<T> From<ops::RangeToInclusive<T>> for RangeType<T> {
    fn from(value: ops::RangeToInclusive<T>) -> Self {
        Self::ToInclusive(value)
    }
}

impl<T> From<ops::Range<T>> for RangeType<T> {
    fn from(value: ops::Range<T>) -> Self {
        Self::Exclusive(value)
    }
}

impl<T> From<ops::RangeTo<T>> for RangeType<T> {
    fn from(value: ops::RangeTo<T>) -> Self {
        Self::ToExclusive(value)
    }
}