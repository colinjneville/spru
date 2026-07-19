use std::{
    any, fmt,
    hash::Hash,
    marker::PhantomData,
    ops,
    sync::atomic::{self, AtomicU32},
};

/// An untyped id which points to an [Item](crate::Item).
/// Untyped ids should only be used to implement [item::Storage](crate::item::Storage).
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
#[repr(transparent)]
#[serde(transparent)]
pub struct Id(u32);

impl Id {
    // TODO Currently required by scripting
    #[doc(hidden)]
    pub fn force_type<T>(self) -> IdT<T> {
        IdT::new(self)
    }

    // For tracing
    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
impl Id {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "i{}", self.0)
    }
}

impl<T> From<IdT<T>> for Id {
    fn from(value: IdT<T>) -> Self {
        value.untyped()
    }
}

/// Strongly-typed identifier for an [Item](crate::Item<T>).
/// As items are atomic unit in a game, they cannot contain one another,
/// they must reference each other using `IdT`s. These ids are passed to
/// an [crate::Interactor] to access the referenced item. `IdT`s should
/// always be used in a game implementation, [Id]s should only be used
/// for [item::Storage](crate::item::Storage) implementations.
#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct IdT<T> {
    id: Id,
    #[serde(skip)]
    _p: PhantomData<T>,
}

impl<T> IdT<T> {
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id,
            _p: PhantomData,
        }
    }

    /// Convert to an untyped id
    pub fn untyped(&self) -> Id {
        self.id
    }
}

#[cfg(feature = "test-util")]
#[doc(hidden)]
impl<T> IdT<T> {
    pub fn test_new(id: u32) -> Self {
        Self::new(Id(id))
    }

    pub fn test_zero() -> Self {
        Self::test_new(0)
    }
}

impl<T> ops::Deref for IdT<T> {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<T> Clone for IdT<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for IdT<T> {}

impl<T> fmt::Debug for IdT<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdT")
            .field("id", &self.id)
            .field("T", &any::type_name::<T>())
            .finish()
    }
}

impl<T> PartialEq for IdT<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> PartialEq<Id> for IdT<T> {
    fn eq(&self, other: &Id) -> bool {
        self.id == *other
    }
}

impl<T> Eq for IdT<T> {}

impl<T> Hash for IdT<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Range {
    range: std::ops::Range<u32>,
}

impl Range {
    pub fn new(range: std::ops::Range<u32>) -> Self {
        Self { range }
    }

    pub fn start(&self) -> Id {
        Id(self.range.start)
    }

    pub fn end(&self) -> Id {
        Id(self.range.end)
    }

    pub(crate) fn contains(&self, id: Id) -> bool {
        self.range.contains(&id.0)
    }

    pub(crate) fn reservation(&self) -> Reservation {
        Reservation {
            next_id: AtomicU32::new(self.range.start),
            end_of_id_reservation: self.range.end,
        }
    }

    /// Advance the range to the beginning start.
    /// Panics if `id` is not within the range.
    pub(crate) fn skip(mut self, id: Id) -> Self {
        assert!(id.0 >= self.range.start);
        assert!(id.0 < self.range.end);

        self.range.start = id.0;
        self
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.range.start, self.range.end)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Reservation {
    next_id: AtomicU32,
    /// Exclusive bound
    end_of_id_reservation: u32,
}

impl Reservation {
    pub(crate) fn all() -> Self {
        Self {
            next_id: AtomicU32::new(0),
            end_of_id_reservation: u32::MAX,
        }
    }

    pub(crate) fn range(&self) -> Range {
        let start = self.next_id.load(atomic::Ordering::Acquire);
        Range::new(start..self.end_of_id_reservation)
    }

    pub(crate) fn claim_id(&self) -> Option<Id> {
        loop {
            let current_value = self.next_id.load(atomic::Ordering::Acquire);
            if current_value >= self.end_of_id_reservation {
                return None;
            }
            if let Ok(current) = self.next_id.compare_exchange(
                current_value,
                current_value + 1,
                atomic::Ordering::AcqRel,
                atomic::Ordering::Acquire,
            ) {
                return Some(Id(current));
            }
        }
    }

    pub(crate) fn split(&mut self, new_reservation_count: u32) -> Result<(Self, Range), ()> {
        if self.end_of_id_reservation >= new_reservation_count {
            let end_of_id_reservation = self.end_of_id_reservation;
            self.end_of_id_reservation -= new_reservation_count;

            let next_id = self
                .end_of_id_reservation
                .max(self.next_id.load(atomic::Ordering::Acquire));

            if next_id < end_of_id_reservation {
                let range = Range::new(next_id..end_of_id_reservation);
                let reservation = Self {
                    next_id: AtomicU32::new(next_id),
                    end_of_id_reservation,
                };
                Ok((reservation, range))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}
