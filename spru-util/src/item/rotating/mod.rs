use std::marker::PhantomData;

use amass::amass_telety;
use perfect_derive::perfect_derive;

#[perfect_derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Catalog)]
#[catalog(error = Error::<T>)]
#[amass_telety(crate::item::rotating)]
pub enum Catalog<T> {
    Create(Create<T>),
    Rotate(Rotate<T>),
    SetPosition(SetPosition<T>),
    Destroy(Destroy<T>),
}

#[perfect_derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Rotating<T> {
    items: Vec<T>,
    position: usize,
}

impl<T> Rotating<T> {
    pub fn create(items: Vec<T>, position: usize) -> Create<T> {
        assert!(position < items.len());

        Create {
            items,
            position,
        }
    }

    pub fn rotate(&self, reverse: bool) -> Rotate<T> {
        Rotate {
            reverse,
            _p: PhantomData,
        }
    }

    pub fn set_position(&self, position: usize) -> SetPosition<T> {
        SetPosition {
            position,
            _p: PhantomData,
        }
    }

    // TODO dynamic insert/remove

    pub fn destroy(&self) -> Destroy<T> {
        Destroy::default()
    }

    pub fn as_slice(&self) -> &[T] {
        self.items.as_slice()
    }
}

#[derive(Debug, Clone)]
#[perfect_derive(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::create(Undo = Destroy<T>)]
pub struct Create<T> {
    items: Vec<T>,
    position: usize,
}

impl<T> spru::Action for Create<T>
where 
    Self: Clone + spru::Serial,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, _input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Self {
            items,
            position,
        } = self.clone();

        Ok((Destroy::default(), Rotating { items, position }))
    }
}

#[perfect_derive(Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::destroy(Undo = Create<T>)]
pub struct Destroy<T>(PhantomData<T>);

impl<T> spru::Action for Destroy<T> 
where 
    Self: Clone + spru::Serial,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Rotating {
            items,
            position: index,
        } = input;

        Ok(Create { items, position: index })
    }
}

#[perfect_derive(Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update]
pub struct Rotate<T> {
    reverse: bool,
    _p: PhantomData<T>,
}

impl<T> spru::Action for Rotate<T>
where 
    Self: Clone + spru::Serial,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Self {
            reverse,
            _p,
        } = *self;

        let len = input.items.len();
        if len > 0 {
            let old_index = input.position;
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
                input.position = new_index;
                return Ok(Some(Self {
                    reverse: !reverse,
                    _p,
                }));
            }
        }

        Ok(None)
    }
}

#[perfect_derive(Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Error = Error<T>)]
pub struct SetPosition<T> {
    position: usize,
    _p: PhantomData<T>,
}

impl<T> spru::Action for SetPosition<T>
where
    Self: Clone + spru::Serial,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Self {
            mut position,
            _p,
        } = *self;

        if position >= input.items.len() {
            Err(Error::InvalidPosition(position))
        } else if position != input.position {
            std::mem::swap(&mut input.position, &mut position);

            Ok(Some(Self {
                position,
                _p,
            }))
        } else {
            Ok(None)
        }
    }
}

#[perfect_derive(Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = Remove<T>, Error = Error<T>)]
pub struct Insert<T> {
    position: usize,
    item: T,
    set_to_inserted: bool,
}

impl<T> spru::Action for Insert<T>
where
    Self: Clone + spru::Serial,
    T: Clone,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Self {
            position,
            ref item,
            set_to_inserted,
        } = *self;

        if position <= input.items.len() {
            input.items.insert(position, item.clone());

            if set_to_inserted {
                // This is the undo of a Remove of the current item
                input.position = position;
            } else if position >= input.position {
                input.position += 1;
            }

            Ok(Remove { position, _p: PhantomData })
        } else {
            Err(Error::InvalidPosition(position))
        }
    }
}

#[perfect_derive(Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[spru::update(Undo = Insert<T>, Error = Error<T>)]
pub struct Remove<T> {
    position: usize,
    _p: PhantomData<fn(T) -> T>,
}

impl<T> spru::Action for Remove<T>
where 
    Self: Clone + spru::Serial,
{
    type T = Rotating<T>;

    fn apply<'l, Lookup>(&self, mut input: spru::action::In<'l, Self, Lookup>) 
        -> Result<impl Into<spru::action::Output<Self::Undo, spru::action::Out<Self, Lookup>>>, Self::Error>
    where 
        Lookup: spru::item::lookup::OfTypeMut<Self::T> + 'l 
    {
        let Self {
            position,
            _p,
        } = *self;

        if position < input.items.len() {
            let set_to_inserted = position == input.position;
            if position < input.position {
                input.position -= 1;
            }
            let item = input.items.remove(position);
            if input.position == input.items.len() {
                input.position = 0;
            }
            Ok(Insert { position, item, set_to_inserted })
        } else {
            Err(Error::InvalidPosition(position))
        }
    }
}

#[perfect_derive(Debug)]
#[derive(spru::FromInfallible)]
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error<T> {
    #[error("Invalid position {0}")]
    InvalidPosition(usize),
    #[doc(hidden)]
    #[error("Unreachable")]
    Phantom(PhantomData<T>),
}