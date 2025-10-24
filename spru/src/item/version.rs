use core::fmt;

use crate::item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(deku::DekuRead, deku::DekuWrite)]
pub struct Version(u32);

impl Version {
    pub(crate) const ZERO: Self = Self(0);
    pub(crate) const INVALID: Self = Self(u32::MAX);

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug)]
#[derive(thiserror::Error)]
#[error("Item {item} expected {expected}, found {actual}")]
pub struct Error {
    pub item: item::Id,
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

    pub fn undo(self) -> Self {
        Self::new(self.after, self.before)
    }

    // Once a version change has been applied to an item, future modifications which are part of the
    // same transaction can leave the version as-is
    pub fn into_noop(self) -> Self {
        Self::noop(self.after)
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

    /// Find the first difference between two `Expected`, if any
    pub(crate) fn diff(&self, actual: &Self) -> Result<(), item::version::Error> {
        // item::version::Error wasn't made for potentially intra-item
        let mut actual_iter = actual.expected.iter().copied();
        for (expected_id, expected_version) in self.expected.iter().copied() {
            let (actual_id, mut actual_version) = actual_iter.next()
                .unwrap_or((item::Id::INVALID, item::Version::INVALID));

            if actual_id != expected_id {
                actual_version = item::Version::INVALID;
            }

            if actual_version != expected_version {
                return Err(item::version::Error {
                    item: expected_id,
                    expected: expected_version,
                    actual: actual_version,
                });
            }
        }

        if let Some((actual_id, actual_version)) = actual_iter.next() {
            Err(item::version::Error {
                item: actual_id,
                expected: item::Version::INVALID,
                actual: actual_version,
            })
        } else {
            Ok(())
        }
    }
}


