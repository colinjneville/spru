use core::fmt;

use crate::item;

/// The version of an [Item](crate::Item). Every transaction that modifies an Item increments its
/// version number by 1. When a [Client](crate::Client) sends an [Interaction](trait@crate::Interaction) to the
/// [Server](crate::Server) to confirm, it sends all the Versions of Items it read. If the server
/// has a different on any of those items, it rejects the Interaction, either because it
/// conflicts, or was based on outdated information.
/// Users never need to manually manage Versions, they are provided for diagnostic purposes only.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    deku::DekuRead,
    deku::DekuWrite,
)]
pub struct Version(u32);

impl Version {
    pub(crate) const ZERO: Self = Self(0);

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct Change {
    pub before: Version,
    pub after: Version,
}

impl Change {
    pub fn new(before: Version, after: Version) -> Self {
        Self { before, after }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct Expected {
    pub(crate) expected: Vec<(item::Id, Version)>,
}

impl Expected {
    pub(crate) fn new(versions: impl Iterator<Item = (item::Id, Version)>) -> Self {
        let mut expected: Vec<_> = versions.collect();
        expected.sort_by_key(|(id, _)| *id);
        Self { expected }
    }

    /// Find the first difference between two [Expected], if any
    pub(crate) fn diff(&self, actual: &Self) -> Result<(), item::Error> {
        let mut actual_iter = actual.expected.iter().copied();
        for (expected_id, expected_version) in self.expected.iter().copied() {
            if let Some((actual_id, actual_version)) = actual_iter.next()
                && actual_id <= expected_id
            {
                if actual_id < expected_id {
                    return Err(item::Error::unexpected_change(actual_id));
                }

                if actual_version != expected_version {
                    return Err(item::Error::wrong_version(
                        expected_id,
                        expected_version,
                        actual_version,
                    ));
                }
            } else {
                return Err(item::Error::expected_change(expected_id));
            };
        }

        if let Some((actual_id, _actual_version)) = actual_iter.next() {
            Err(item::Error::unexpected_change(actual_id))
        } else {
            Ok(())
        }
    }
}
