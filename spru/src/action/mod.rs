pub mod error;
pub use error::Error;

use crate::{Item, action, common::error::AnyResult, item};

pub use spru_macro::{Create, Destroy, Update};
use tagset::tagset_meta;

/// Implemented automatically by the Create/Update/Destroy macros
#[doc(hidden)]
pub trait SubAction {
    type Undo;
    type T;

    fn apply<Storage>(&self, context: Context<'_, Storage>) -> action::Result<Option<Self::Undo>>
    where
        Storage: item::Storage,
        Self::T: item::storage::Storable<Storage::State>;

    fn apply_map<Storage, Action>(
        &self,
        context: Context<'_, Storage>,
    ) -> action::Result<Option<Action>>
    where
        Storage: item::Storage,
        Self::T: item::storage::Storable<Storage::State>,
        Self::Undo: Into<Action>,
    {
        self.apply(context)
            .map(|o| o.map(Into::into))
            .map_err(|e| e.with_context(self))
    }
}

/// The set of atomic changes which can be made to the [State](trait@crate::State) objects.  
///
/// Actions consist of [Create], [Update], and [Destroy] types. To define a type as an action,
/// derive one of these traits for it, *and* implement the trait (the derive implements a wrapper
/// to unify Create/Update/Destroy under a single internal trait).
///
/// # Implementing Create/Update/Destroy
/// Each of these traits has a type parameter `T`, indicating the type of State it operates on.
/// Type parameter `Undo` specifies a type which can be used to revert the item to its previous state.
/// For `Create`, this will be a `Destroy`, and vice versa. For `Update`, this will be an `Update`
/// (possibly even `Self`, depending on the action). If an action is not successful, the effective
/// game state must be unchanged, and the action returns an error. See [Create], [Update], and [Destroy]
/// for more details.  
///
/// Here's an example of an [Update] action that adds a value to an `i32` item:  
/// ```rust
/// // Both `derive` and `impl` are required
/// #[derive(spru::action::Update)]
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct AddI32(pub i32);
///
/// impl spru::action::Update for AddI32 {
///     // This action applies to a i32 value
///     type T = i32;
///     // We can reuse the same action type by negating the value when undoing
///     type Undo = AddI32;
///
///     fn update(&self, value: &mut Self::T) -> spru::common::error::AnyResult<impl Into<Option<Self::Undo>>> {
///         // Note: This example is not overflow-safe
///         // An update modifies an existing `value`
///         *value += self.0;
///         // Adding the negated value effectively undoes it
///         Ok(Some(AddI32(-self.0)))
///     }
/// }
/// ```
/// # cloned Actions
/// The crate `spru_util` contains `cloned::Create<T>`, `cloned::Update<T>`, and `cloned::Destroy<T>`.
/// These actions modify the item through serialization. This is usually all you need to Create and Destroy,
/// but may be inefficient or semantically lacking for updates.
///
/// # Implementing Action
/// Action is implemented using [tagset::tagset] to create a tagged union of all possible actions. You
/// must specify the [State](trait@crate::State) set this action set applies to:
/// ```rust,ignore
/// #[tagset(impl spru::Action {
///     type State = MyState;
/// })]
/// ```
/// Here is an example of a game with a State set of `{ MyObject }`, and Actions `{ MyCreate, MyDestroy }`:
/// ```rust,ignore
/// # use tagset::tagset;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct MyObject(i32);
///
/// #[tagset(impl tagset::proxy::serde::Serialize)]
/// #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
/// #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
/// #[tagset(impl spru::State)]
/// #[tagset(MyObject)]
/// pub struct MyState;
///
/// #[derive(spru::action::Create)]
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct MyCreate {
///     value: i32,
/// }
/// impl spru::action::Create for MyCreate {
///     type T = MyObject;
///     type Undo = MyDestroy;
///
///     fn create(&self) -> spru::common::error::AnyResult<(Self::T, Self::Undo)> {
///         Ok((MyObject(self.value), MyDestroy { }))
///     }
/// }
///
/// #[derive(spru::action::Destroy)]
/// #[derive(serde::Serialize, serde::Deserialize)]
/// pub struct MyDestroy;
///
/// impl spru::action::Destroy for MyDestroy {
///     type T = MyObject;
///     type Undo = MyCreate;
///
///     fn destroy(&self, value: Self::T) -> spru::common::error::AnyResult<Self::Undo> {
///         Ok(MyCreate { value: value.0 })
///     }
/// }
///
/// #[tagset(impl spru::Action {
///     type State = MyState;
/// })]
/// #[tagset(impl tagset::proxy::serde::Serialize)]
/// #[tagset(impl<'de> tagset::serde::DeserializeFromDiscriminant<'de>)]
/// #[tagset(impl<'de> tagset::proxy::serde::Deserialize<'de>)]
/// #[tagset(derive(Debug, Clone))]
/// #[tagset(MyCreate)]
/// #[tagset(MyDestroy)]
/// pub struct MyAction;
///
/// # fn main() { }
/// ```
#[telety::telety(crate::action, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(
    for<VAR> VAR:
        crate::action::SubAction<Undo: Into<Self>>,
))]
pub trait Action: Sized + 'static {
    /// The type of item this action applies to
    type State: crate::State;

    #[doc(hidden)]
    #[meta(default {
        match_by_value!(self, v => spru::action::SubAction::apply_map(v, context))
    })]
    fn apply<Storage: item::Storage<State = Self::State>>(
        &self,
        context: Context<'_, Storage>,
    ) -> action::Result<Option<Self>>;
}

/// An action which creates a new item. Use `#[derive(spru::action::Create)]` *and*
/// `impl` this trait.
pub trait Create {
    /// The type of item this action applies to
    type T;
    /// The [Destroy] which undoes this create
    type Undo;

    /// Create the item, returning a tuple of the created item and the undo action
    /// which destroys it.
    fn create(&self) -> AnyResult<(Self::T, Self::Undo)>;
}

/// An action which updates an existing item. Use `#[derive(spru::action::Update)]` *and*
/// `impl` this trait.
pub trait Update {
    /// The type of item this action applies to
    type T;
    /// The [Update] which undoes this update. May be `Self`.
    type Undo;

    /// Update an existing item. If no change is needed, (e.g. an update to add `0`),
    /// return `Ok(None)`, and the update will be skipped entirely.
    fn update(&self, value: &mut Self::T) -> AnyResult<impl Into<Option<Self::Undo>>>;
}

/// An action which destroys an existing item. Use `#[derive(spru::action::Destroy)]` *and*
/// `impl` this trait.
pub trait Destroy {
    /// The type of item this action applies to
    type T;
    /// The [Create] which undoes this destroy
    type Undo;

    /// Destroy the item (usually just allowing it to drop).
    /// Return a [Create] action which recreate it in the state it was destroyed.
    fn destroy(&self, value: Self::T) -> AnyResult<Self::Undo>;
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Context<'l, Storage> {
    pub(crate) storage: &'l mut Storage,
    pub(crate) id: item::Id,
    pub(crate) version: item::version::Change,
}

impl<'l, Storage> Context<'l, Storage> {
    pub(crate) fn new(
        storage: &'l mut Storage,
        id: item::Id,
        version: item::version::Change,
    ) -> Self {
        Self {
            storage,
            id,
            version,
        }
    }

    #[doc(hidden)]
    pub fn create<C>(self, c: &C) -> action::Result<Option<C::Undo>>
    where
        Storage: item::Storage,
        C: Create<T: item::storage::Storable<Storage::State>>,
    {
        let Self {
            storage,
            id,
            version,
        } = self;

        if let Ok(item) = storage.get(item::IdT::<C::T>::new(id)) {
            Err(action::Error::new_item(item::Error::already_exists(
                id,
                item.version(),
            )))
        } else {
            let (value, undo) = c.create()?;

            let item = Item::new(item::IdT::new(id), version.after, value);
            storage.create(item)?;

            Ok(Some(undo))
        }
    }

    #[doc(hidden)]
    pub fn update<U>(self, u: &U) -> action::Result<Option<U::Undo>>
    where
        Storage: item::storage::Storage,
        U: Update<T: item::storage::Storable<Storage::State>>,
    {
        let Self {
            storage,
            id,
            version,
        } = self;

        let id = item::IdT::<U::T>::new(id);
        let mut value = storage.get_mut(id).map_err(|e| e.with_context(id))?;
        if version.before == (*value).version() {
            let undo = u.update(value.get_mut()).map(Into::into)?;

            // Only update the version on the first non-noop update. If this Item
            // is not modified at all during the transaction we can't bump the version number.
            if undo.is_some() {
                (*value).set_version(version.after);
            }

            Ok(undo)
        } else {
            Err(action::Error::new_item(item::Error::wrong_version(
                id.untyped(),
                version.before,
                value.version(),
            )))
        }
    }

    #[doc(hidden)]
    pub fn destroy<D>(self, d: &D) -> action::Result<Option<D::Undo>>
    where
        Storage: item::storage::Storage,
        D: Destroy<T: item::storage::Storable<Storage::State>>,
    {
        let Self {
            storage,
            id,
            version,
        } = self;

        let id = item::IdT::<D::T>::new(id);
        let item = storage.get(id).map_err(|e| e.with_context(id))?;
        if version.before == item.version() {
            let item = storage.destroy(id)?;
            let undo = d.destroy(item.into_value())?;
            Ok(Some(undo))
        } else {
            Err(action::Error::new_item(item::Error::wrong_version(
                id.untyped(),
                version.before,
                item.version(),
            )))
        }
    }
}

/// A result with an [Error] `Err`
pub type Result<T> = std::result::Result<T, self::Error>;
