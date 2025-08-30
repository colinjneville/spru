use core::fmt;

use crate::item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(deku::DekuRead, deku::DekuWrite)]
pub struct Version(u32);

impl Version {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn previous(&self) -> Option<Self> {
        if self.0 > 0 {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }

    pub(crate) fn subsequent(&self, is_undo: bool) -> Option<Self> {
        if is_undo {
            self.previous()
        } else {
            Some(self.next())
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Expected {expected}, found {actual}")]
pub struct Error {
    pub expected: Version,
    pub actual: Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Change {
    pub before: Version,
    pub after: Version,
}

impl Change {
    pub fn new(before: Version, after: Version) -> Self {
        Self {
            before, 
            after,
        }
    }

    pub fn noop(version: Version) -> Self {
        Self::new(version, version)
    }

    pub fn create() -> Self {
        Self::new(Version::ZERO, Version::ZERO.next())
    }

    pub fn update(before: Version) -> Self {
        Self::new(before, before.next())
    }

    pub fn destroy(before: Version) -> Self {
        Self::new(before, before.next())
    }

    pub fn undo(&self) -> Self {
        Self::new(self.after, self.before)
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.before, self.after)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Expected {
    pub(crate) expected: Vec<(item::Id, Version)>,
}

impl Expected {
    pub(crate) fn new(versions: impl Iterator<Item = (item::Id, Version)>) -> Self {
        let mut expected: Vec<_> = versions.collect();
        expected.sort_by_key(|(id, _)| *id);
        Self { expected }
    }
}


