use std::{fmt, ops::RangeBounds as _};

#[derive(Debug, Clone, thiserror::Error)]
pub struct ValueOutOfBounds<T> {
    pub value: T,
    pub bounds: super::RangeType<T>,
}

impl<T> ValueOutOfBounds<T> {
    pub(crate) fn new(value: T, bounds: super::RangeType<T>) -> Self {
        Self {
            value,
            bounds,
        }
    }
}

impl<T> fmt::Display for ValueOutOfBounds<T> 
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            value,
            bounds,
        } = self;

        let start = match bounds.start_bound() {
            std::ops::Bound::Included(b) => format!("{b}="),
            std::ops::Bound::Excluded(b) => b.to_string(),
            std::ops::Bound::Unbounded => String::new(),
        };

        let end = match bounds.end_bound() {
            std::ops::Bound::Included(b) => format!("={b}"),
            std::ops::Bound::Excluded(b) => b.to_string(),
            std::ops::Bound::Unbounded => String::new(),
        };

        write!(f, "Value '{value}' is not within bounds '{start}..{end}'")?;
        Ok(())
    }
}
