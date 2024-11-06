use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Id(pub(crate) usize);

impl Id {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) const fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn get(&self) -> usize {
        self.0
    }
}

#[cfg(feature = "test-util")]
impl Id {
    pub fn test_new(id: usize) -> Self {
        Self(id)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Expected {expected}, found {actual}")]
pub struct Error {
    expected: Id,
    actual: Id,
}

impl Error {
    pub(crate) fn new(expected: Id, actual: Id) -> Self {
        Self {
            expected,
            actual,
        }
    }
}