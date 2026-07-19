pub mod error;

use std::marker::PhantomData;

use derive_where::derive_where;
use spru::common::error::AnyResult;
use spru_script::script;
use tagset::tagset;
use telety::telety;

use crate::cloned;

/// A 'wheel' of values which loops around at both ends. Can represent turn order, round phases, etc.
#[derive(Debug, Clone)]
#[derive_where(Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[script(include = [Methods])]
pub struct Rotating<T> {
    // #[get]
    items: Vec<T>,
    // #[get]
    position: usize,
}

#[script(partial = Methods)]
impl<T: Clone + 'static> Rotating<T> {
    #[create]
    fn create(items: Vec<T>, position: usize) -> Create<T> {
        create(items,position)
    }

    #[create]
    fn dflt() -> Create<T> {
        create(vec![], 0)
    }

    #[method]
    fn rotate(&self) -> ((), Rotate<T>) {
        ((), rotate(false))
    }

    #[method]
    fn rotate_reverse(&self) -> ((), Rotate<T>) {
        ((), rotate(true))
    }

    #[get(name = len)]
    fn _len(&self) -> usize {
        self.items.len()
    }

    #[get(name = is_empty)]
    fn _is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[get(name = position)]
    fn _position(&self) -> Option<usize> {
        self.position()
    }

    #[set(name = position)]
    fn position_set(&self, position: usize) -> (SetPosition<T>, ) {
        (set_position(position), )
    }

    #[get(name = current)]
    fn _current(&self) -> Option<T> {
        self.items.get(self.position).cloned()
    }

    #[get]
    fn items(&self) -> Vec<T> {
        self.items.clone()
    }

    #[set(name = items)]
    fn items_set(&self, items: Vec<T>) -> (SetItems<T>, ) {
        (set_items(items), )
    }

    #[method]
    fn destroy(&self) -> ((), Destroy<T>) {
        ((), cloned::destroy())
    }

    #[method(name = insert)]
    fn _insert(&self, position: usize, item: T) -> ((), Insert<T>) {
        ((), insert(position, item))
    }

    #[method(name = remove)]
    fn _remove(&self, position: usize) -> (Option<T>, Remove<T>) {
        (self.items.get(position).cloned(), remove(position))
    }

    #[method(name = push)]
    fn _push(&self, item: T) -> ((), Insert<T>) {
        self._insert(self.len(), item)
    }

    #[method(name = pop)]
    fn _pop(&self) -> (Option<T>, Remove<T>) {
        self._remove(self.len().saturating_sub(1))
    }
}

impl<T> Rotating<T> {
    pub fn as_slice(&self) -> &[T] {
        self.items.as_slice()
    }

    /// The index of the selected value. [None] if this item has no elements.
    pub fn position(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.position)
        }
    }

    /// The current selected value. [None] if this item has no elements.
    pub fn current(&self) -> Option<&T> {
        self.items.get(self.position)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub fn create<T>(items: Vec<T>, position: usize) -> Create<T> {
    assert!(items.is_empty() || position < items.len());

    cloned::create(Rotating { items, position })
}

pub fn default<T>() -> Create<T> {
    create(vec![], 0)
}

pub fn set_items<T>(items: Vec<T>) -> SetItems<T> {
    SetItems {
        items,
        position: None, 
    }
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
    cloned::destroy()
}

#[telety(crate::rotating)]
#[tagset(Create<T>)]
#[tagset(SetItems<T>)]
#[tagset(Rotate<T>)]
#[tagset(SetPosition<T>)]
#[tagset(Insert<T>)]
#[tagset(Remove<T>)]
#[tagset(Destroy<T>)]
#[tagset(reserved(..16))]
pub struct Actions<T>;

pub type Create<T> = cloned::Create<Rotating<T>>;

pub type Destroy<T> = cloned::Destroy<Rotating<T>>;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct SetItems<T> {
    items: Vec<T>,
    position: Option<usize>,
}

impl<T> spru::action::Update for SetItems<T>
where
    T: Clone,
{
    type T = Rotating<T>;
    type Undo = Self;

    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>> {
        
        let items = std::mem::replace(&mut value.items, self.items.clone());
        let p = match self.position {
            Some(p) => p,
            None => value.position.clamp(1, value.items.len()) - 1,
        };

        let position = if p != value.position {
            Some(std::mem::replace(&mut value.position, p))
        } else {
            None
        };
        
        Ok(Self {
            items,
            position,
        })
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Rotate<T> {
    reverse: bool,
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Rotate<T>
where
    T: Clone,
{
    type T = Rotating<T>;
    type Undo = Self;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        let Self { reverse, _p } = *self;

        let len = value.items.len();
        if len > 0 {
            let old_index = value.position;
            let new_index = if reverse {
                if old_index == 0 {
                    len - 1
                } else {
                    old_index - 1
                }
            } else if old_index == len - 1 {
                0
            } else {
                old_index + 1
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
    T: Clone,
{
    type T = Rotating<T>;
    type Undo = Self;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Option<Self::Undo>> {
        let Self { mut position, _p } = *self;

        if position >= value.items.len() {
            Err(error::IndexOutOfRange::new(position, value.items.len()).into())
        } else if position != value.position {
            std::mem::swap(&mut value.position, &mut position);

            Ok(Some(Self { position, _p }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, spru::action::Update)]
#[must_use]
pub struct Insert<T> {
    position: usize,
    item: T,
    set_to_inserted: bool,
}

impl<T> spru::action::Update for Insert<T>
where
    T: Clone,
{
    type T = Rotating<T>;
    type Undo = Remove<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
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
            } else if value.position >= position && value.items.len() > 1 {
                // If this is inserted before the current selection (and we had an existing selection
                // because we weren't empty), bump the index to maintain the same selected item
                value.position += 1;
            }

            Ok(Remove {
                position,
                _p: PhantomData,
            })
        } else {
            Err(error::IndexOutOfRange::new(position, value.items.len()).into())
        }
    }
}

#[derive_where(Debug, Clone, Default, Serialize, Deserialize)]
#[derive(spru::action::Update)]
#[must_use]
pub struct Remove<T> {
    position: usize,
    #[serde(skip)]
    _p: PhantomData<T>,
}

impl<T> spru::action::Update for Remove<T>
where
    T: Clone,
{
    type T = Rotating<T>;
    type Undo = Insert<T>;

    #[allow(refining_impl_trait)]
    fn update(&self, value: &mut Self::T) -> AnyResult<Self::Undo> {
        let Self { position, _p } = *self;

        if position < value.items.len() {
            let set_to_inserted = position == value.position;
            if position < value.position {
                value.position -= 1;
            }
            let item = value.items.remove(position);
            if value.position == value.items.len() {
                value.position = 0;
            }
            Ok(Insert {
                position,
                item,
                set_to_inserted,
            })
        } else {
            Err(error::IndexOutOfRange::new(position, value.items.len()).into())
        }
    }
}
