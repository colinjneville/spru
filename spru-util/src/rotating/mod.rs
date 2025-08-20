use std::marker::PhantomData;

use derive_where::derive_where;
use tagset::tagset;
use telety::telety;

use crate::verbatim;

#[telety(crate::rotating)]
#[derive(Debug, Clone)]
#[derive_where(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct State<T> {
    items: Vec<T>,
    position: usize,
}

impl<T> State<T> {
    pub fn as_slice(&self) -> &[T] {
        self.items.as_slice()
    }

    pub fn position(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.position)
        }
    }

    pub fn current(&self) -> Option<&T> {
        self.items.get(self.position)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub fn create<T>(items: Vec<T>, position: usize) -> Create<T> {
    assert!(position < items.len());

    verbatim::create(
        State {
            items,
            position,
        }
    )
}

pub fn default<T>() -> Create<T> {
    create(vec![], 0)
}

pub fn rotate<T>(reverse: bool) -> Rotate<T> {
    Rotate {
        reverse,
        _p: PhantomData,
    }
}

pub fn set_position<T>(position: usize) -> SetPosition<T> {
    SetPosition {
        position,
        _p: PhantomData,
    }
}

pub fn insert<T>(position: usize, item: T) -> Insert<T> {
    Insert {
        position,
        item,
        set_to_inserted: false,
    }
}

pub fn remove<T>(position: usize) -> Remove<T> {
    Remove {
        position,
        _p: PhantomData,
    }
}

pub fn destroy<T>() -> Destroy<T> {
    verbatim::destroy()
}

#[telety(crate::rotating)]
#[tagset(Create<T>)]
#[tagset(Rotate<T>)]
#[tagset(SetPosition<T>)]
#[tagset(Insert<T>)]
#[tagset(Remove<T>)]
#[tagset(Destroy<T>)]
#[tagset(reserved(..16))]
pub struct Actions<T>;

pub type Create<T> = verbatim::Create<State<T>>;

pub type Destroy<T> = verbatim::Destroy<State<T>>;

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Rotate<T> {
    reverse: bool,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Rotate<T>
where 
    Self: Clone + spru::Serial,
{
    type T = State<T>;
    type Undo = Self;
    type Error = std::convert::Infallible;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let Self {
            reverse,
            _p,
        } = *self;

        let len = value.items.len();
        if len > 0 {
            let old_index = value.position;
            let new_index = if reverse {
                if old_index == 0 {
                    len - 1
                } else {
                    old_index - 1
                }
            } else {
                if old_index == len - 1 {
                    0
                } else {
                    old_index + 1
                }
            };
            
            if old_index != new_index {
                value.position = new_index;
                return Ok(Some(Self {
                    reverse: !reverse,
                    _p,
                }));
            }
        }

        Ok(None)
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct SetPosition<T> {
    position: usize,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for SetPosition<T>
where
    Self: Clone + spru::Serial,
{
    type T = State<T>;
    type Undo = Self;
    type Error = Error;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let Self {
            mut position,
            _p,
        } = *self;

        if position >= value.items.len() {
            Err(Error::InvalidPosition(position))
        } else if position != value.position {
            std::mem::swap(&mut value.position, &mut position);

            Ok(Some(Self {
                position,
                _p,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Insert<T> {
    position: usize,
    item: T,
    set_to_inserted: bool,
}

impl<T> spru::action::Update for Insert<T>
where
    Self: Clone + spru::Serial,
    T: Clone,
{
    type T = State<T>;
    type Undo = Remove<T>;
    type Error = Error;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let Self {
            position,
            ref item,
            set_to_inserted,
        } = *self;

        if position <= value.items.len() {
            value.items.insert(position, item.clone());

            if set_to_inserted {
                // This is the undo of a Remove of the current item
                value.position = position;
            } else if position >= value.position {
                value.position += 1;
            }

            Ok(Some(Remove { position, _p: PhantomData }))
        } else {
            Err(Error::InvalidPosition(position))
        }
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Remove<T> {
    position: usize,
    _p: PhantomData<fn(T) -> T>,
}

impl<T> spru::action::Update for Remove<T>
where 
    Self: Clone + spru::Serial,
{
    type T = State<T>;
    type Undo = Insert<T>;
    type Error = Error;

    fn update(&self, value: &mut Self::T) -> Result<Option<Self::Undo>, Self::Error> {
        let Self {
            position,
            _p,
        } = *self;

        if position < value.items.len() {
            let set_to_inserted = position == value.position;
            if position < value.position {
                value.position -= 1;
            }
            let item = value.items.remove(position);
            if value.position == value.items.len() {
                value.position = 0;
            }
            Ok(Some(Insert { position, item, set_to_inserted }))
        } else {
            Err(Error::InvalidPosition(position))
        }
    }
}

#[derive(Debug, Clone)]
#[derive(spru::FromInfallible)]
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Invalid position {0}")]
    InvalidPosition(usize),
}