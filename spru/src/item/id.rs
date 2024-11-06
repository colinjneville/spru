use std::{any, fmt, hash::Hash, marker::PhantomData, ops, sync::atomic::{self, AtomicUsize}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(deku::DekuRead, deku::DekuWrite)]
pub struct Id(usize);

impl Id {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub(crate) fn force_type<T>(self) -> IdT<T> {
        IdT::new(self)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "i{}", self.0)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct IdT<T> {
    id: Id, 
    #[serde(skip_serializing)]
    _p: PhantomData<fn() -> T>,
}

impl<T> IdT<T> {
    // TODO make public?
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id, 
            _p: PhantomData,
        }
    }

    pub fn untyped(&self) -> &Id {
        &self.id
    }
}

#[cfg(feature = "test-util")]
#[doc(hidden)]
impl<T> IdT<T> {
    pub fn test_new(id: usize) -> Self {
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
        Self { id: self.id.clone(), _p: PhantomData }
    }
}

impl<T> Copy for IdT<T> { }

impl<T> fmt::Debug for IdT<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdT").field("id", &self.id).field("T", &any::type_name::<T>()).finish()
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

impl<T> Eq for IdT<T> { }

impl<T> Hash for IdT<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Range {
    range: std::ops::Range<usize>,
}

impl Range {
    pub fn new(range: std::ops::Range<usize>) -> Self {
        Self {
            range,
        }
    }

    pub(crate) fn contains(&self, id: &Id) -> bool {
        self.range.contains(&id.0)
    }

    pub(crate) fn reservation(&self) -> Reservation {
        Reservation {
            next_id: AtomicUsize::new(self.range.start),
            end_of_id_reservation: self.range.end,
        }
    }
}


#[derive(Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Reservation {
    next_id: AtomicUsize,
    /// Exclusive bound
    end_of_id_reservation: usize,
}

impl Reservation {
    pub(crate) fn all() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
            end_of_id_reservation: usize::MAX,
        }
    }

    pub(crate) fn range(&self) -> Range {
        let start = self.next_id.load(atomic::Ordering::Acquire);
        Range::new(start..self.end_of_id_reservation)
    }

    pub(crate) fn claim_id(&self) -> Result<Id, ()> {
        loop {
            let current_value = self.next_id.load(atomic::Ordering::Acquire);
            if current_value >= self.end_of_id_reservation {
                return Err(());
            }
            if let Ok(current) = self.next_id.compare_exchange(current_value, current_value + 1, atomic::Ordering::AcqRel, atomic::Ordering::Acquire) {
                return Ok(Id(current));
            }
        }
    }

    pub fn split(&mut self, new_reservation_count: usize) -> Result<(Self, Range), ()> {
        if self.end_of_id_reservation >= new_reservation_count {
            let end_of_id_reservation = self.end_of_id_reservation;
            self.end_of_id_reservation -= new_reservation_count;
            
            let next_id = self.end_of_id_reservation
                .max(self.next_id.load(atomic::Ordering::Acquire));
                
            if next_id < end_of_id_reservation {
                let range = Range::new(next_id..end_of_id_reservation);
                let reservation = Self {
                    next_id: AtomicUsize::new(next_id),
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

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum Error {
    #[error("Instance {id} already exists ({version})")]
    AlreadyExists { id: Id, version: super::Version },
    #[error("Instance {id} has already been destroyed")]
    IsDestroyed { id: Id, },
}
