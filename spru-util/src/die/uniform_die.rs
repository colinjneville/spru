use std::ops::RangeInclusive;

pub struct UniformDie<T>(RangeInclusive<T>);