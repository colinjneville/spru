//! Language-agnostic support for scripting in spru. 
//! Game Items should use the included derive macros or implement the `Scriptable*` traits manually, 
//! and specific scripting implementations should implement the `Registry*` and `Registration*` traits.
mod game_init;
pub use game_init::GameInit;
mod interaction;
pub use interaction::Interaction;
mod player_init;
pub use player_init::PlayerInit;
mod reaction;
pub use reaction::Reaction;

use std::ops;

use tagset::tagset_meta;
use telety::telety;


/// A pre-parsed path-wise representation of a type, including type arguments. Create using [scriptable_path].
#[derive(Debug, PartialEq, Eq)]
pub struct ScriptablePath(pub &'static [&'static str], pub &'static [Self]);

/// Implements [ScriptableType] for the type. Apply to the struct definition and/or impl blocks.
/// Apply `#[get]` and `#[set]` attributes to fields, and `#[get]`, `#[set]`, `#[method]`, and `#[create]`
/// attributes to functions in an impl block.
/// TODO further documentation
pub use spru_script_macro::script;

/// Parses a rust type path during macro expansion. Used for implementing [Scriptable::register].
/// Not normally needed for manual use.
pub use spru_script_macro::scriptable_path;

/// A [spru::State] with scripting support. Implement using the [tagset::tagset] macro.
/// ```ignore
/// #[tagset(impl<Action, Registry> ScriptableState<Action, Registry>)]
/// ```
#[telety(crate, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(
    Registry: self::Registry<Self, Action>,
    for<VAR> Registry: self::RegistryState<Self, Action, VAR>,
))]
pub trait Scriptable<Action, Registry: ?Sized>: spru::State 
where
    Registry: self::Registry<Self, Action>,
{
    /// Registers substates with the scripting implementation.
    /// [ScriptRegistryState::register_state] should be called for each applicable type.
    #[meta(default {
        foreach!(VAR => {
            spru_script::RegistryState::<Self, Action, VAR>::register_state::<Storage>(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(VAR))
            )?;
        });
        Ok(())
    })]
    fn register<Storage>(registry: &Registry, registration: &mut Registry::Registration<'_, Storage>) 
        -> Result<(), Registry::Error>
    where
        Storage: spru::item::Storage<State = Self>,
    ;
}


pub trait ScriptableState<State, Action, Registry: ?Sized>: Sized + 'static
where
    Registry: self::Registry<State, Action>,
{
    /// The type being registered, usually `Self`
    type Type;

    /// Registers members of this type with the scripting implementation.
    /// [RegistryStateGet::register_state_get], etc. should be called for each field/method.
    fn register_state<Storage>(
        registry: &Registry, 
        registration: &mut Registry::RegistrationState<'_, Storage, Self::Type>, 
    )
         -> Result<(), Registry::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

pub trait ScriptableType<State, Action, Registry: ?Sized>: Sized + 'static
where
    Registry: self::Registry<State, Action>,
{
    /// The type being registered, usually `Self`
    type Type;

    /// Registers members of this type with the scripting implementation.
    /// [RegistryTypeGet::register_type_get], etc. should be called for each field/method.
    fn register_type(
        registry: &Registry, 
        registration: &mut Registry::RegistrationType<'_, Self::Type>, 
    )
         -> Result<(), Registry::Error>
    ;
}

pub trait LanguageBase<State, Action> {
    type Registry: Registry<State, Action>;
    type Error;
}

pub trait Language<State, Action, Args, Ret, Context, Output>: LanguageBase<State, Action> {
    /// Execute a script.
    fn exec<Storage>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
        args: Args,
    ) -> Result<Ret, Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
        State: spru::State + Scriptable<Action, Self::Registry>,
        Action: spru::Action<State = State>,
    ;
}

/// A scripting implementation. Implementations will also implement [ScriptRegistryType] and [ScriptRegistryGetter], etc. based
/// on the specific types they can support. 
pub trait Registry<State, Action> {
    /// The stateful type responsible for performing registration of States/types based on the concrete Storage implementation.
    type Registration<'r, Storage>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    /// The stateful type responsible for performing field/method registration for States based on the concrete Storage implementation.
    type RegistrationState<'r, Storage, T: 'static>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    /// The stateful type responsible for performing field/method registration for other types
    type RegistrationType<'r, T: 'static>;

    /// The error type for the scripting implementation.
    type Error;
}

/// Scripting implementations implement this for types they support.
pub trait RegistryState<State, Action, T>: Registry<State, Action> {
    /// Register a type and all its fields/methods with the scripting implementation.
    fn register_state<Storage>(
        &self, 
        registration: &mut Self::Registration<'_, Storage>,
        type_path: Option<ScriptablePath>,
    ) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as immutable non-State types (i.e. just `T`, not `IdT<T>`)
pub trait RegistryType<State, Action, T>: Registry<State, Action> {
    /// Register an immutable non-State type.
    fn register_type<Storage>(
        &self, 
        registration: &mut Self::Registration<'_, Storage>, 
        type_path: Option<ScriptablePath>,
    )
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as field getters.
pub trait RegistryStateGet<State, Action, T, U>: Registry<State, Action> {
    /// Register a read-only getter for a field.
    fn register_state_get<Storage>(&self, registration: &mut Self::RegistrationState<'_, Storage, T>, ident: &str, getter: fn(&T) -> U)
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as field setters.
pub trait RegistryStateSet<State, Action, T, U>: Registry<State, Action> {
    /// Register a setter for a field.
    fn register_state_set<Storage>(&self, registration: &mut Self::RegistrationState<'_, Storage, T>, ident: &str, setter: fn(&T, U) -> Vec<Action>) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as methods.
pub trait RegistryStateMethod<State, Action, T, Args, Ret>: Registry<State, Action> {
    /// Register a method.
    fn register_state_method<Storage>(&self, registration: &mut Self::RegistrationState<'_, Storage, T>, ident: &str, method: fn(&T, Args) -> (Ret, Vec<Action>)) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}


/// Scripting implementations implement this for types they support as constructors.
pub trait RegistryStateCreate<State, Action, Create, T, Args>: Registry<State, Action> 
where
    Create: spru::action::Create<T = T> + Into<Action>,
{
    /// Register a constructor.
    fn register_state_create<Storage>(&self, registration: &mut Self::RegistrationState<'_, Storage, T>, ident: &str, create: fn(Args) -> Create) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as functions.
pub trait RegistryStateFunction<State, Action, T, Args, Ret>: Registry<State, Action> {
    /// Register a constructor.
    fn register_state_function<Storage>(&self, registration: &mut Self::RegistrationState<'_, Storage, T>, ident: &str, create: fn(Args) -> Ret) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as field getters.
pub trait RegistryTypeGet<State, Action, T, U>: Registry<State, Action> {
    /// Register a read-only getter for a field.
    fn register_type_get(&self, registration: &mut Self::RegistrationType<'_, T>, ident: &str, getter: fn(&T) -> U)
        -> Result<(), Self::Error>
    ;
}

/// Scripting implementations implement this for types they support as methods.
pub trait RegistryTypeMethod<State, Action, T, Args, Ret>: Registry<State, Action> {
    /// Register a method.
    fn register_type_method(&self, registration: &mut Self::RegistrationType<'_, T>, ident: &str, method: fn(&T, Args) -> Ret)
        -> Result<(), Self::Error>
    ;
}

/// Scripting implementations implement this for types they support as functions.
pub trait RegistryTypeFunction<State, Action, T, Args, Ret>: Registry<State, Action> {
    /// Register a function.
    fn register_type_function(&self, registration: &mut Self::RegistrationType<'_, T>, ident: &str, method: fn(Args) -> Ret)
        -> Result<(), Self::Error>
    ;
}

/// Scripting implementations implement this for types supporting equality functions.
pub trait RegistryTypeEq<State, Action, T>: Registry<State, Action> {
    /// Register an equality function for a type.
    fn register_type_eq(&self, registration: &mut Self::RegistrationType<'_, T>, eq: fn(&T, &T) -> bool)
        -> Result<(), Self::Error>
    ;
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Hash, PartialOrd, Ord)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Wrap<T>(pub T);

// SAFETY Wrap is a publically #[repr(transparent)] wrapper around T
unsafe impl<T> bytemuck::TransparentWrapper<T> for Wrap<T> { }

pub use bytemuck::{TransparentWrapper, TransparentWrapperAlloc};

impl<T> Wrap<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> ops::Deref for Wrap<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for Wrap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<State, Action, Registry: ?Sized, T> ScriptableType<State, Action, Registry> for Wrap<T> 
where
    T: ScriptableType<State, Action, Registry>,
    Registry: crate::Registry<State, Action>,
{
    type Type = T::Type;

    fn register_type(
        registry: &Registry, 
        registration: &mut Registry::RegistrationType<'_, Self::Type>, 
    )
         -> Result<(), Registry::Error>
     {
        T::register_type(registry, registration)
    }
}

mod private {
    #[doc(hidden)]
    pub trait Sealed { }
}

#[doc(hidden)]
/// Used by [script] to allow returning multiple sub-Action types from methods.
pub trait MethodReturn<Action> : private::Sealed {
    type T;

    fn convert(self) -> (Self::T, Vec<Action>);

    fn wrap_convert(self) -> (Wrap<Self::T>, Vec<Action>)
    where 
        Self: Sized
    {
        let (ret, actions) = self.convert();
        (Wrap(ret), actions)
    }
}

macro_rules! tuple_method_return {
    () => { };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        impl<T, $first, $($rest),*> private::Sealed for (T, $first, $($rest),*) { }

        impl<T, Action, $first, $($rest),*> MethodReturn<Action> for (T, $first, $($rest),*) 
        where
            $first: Into<Action>,
            $($rest: Into<Action>),*
        {
            type T = T;

            fn convert(self) -> (Self::T, Vec<Action>) {
                let mut v = vec![
                    self.$n.into(),
                    $(self.$nn.into()),*
                ];
                
                v.reverse();

                (self.0, v)
            }
        }
        tuple_method_return!($($nn $rest)*);
    };
}

impl<T> private::Sealed for (T, ) { }

impl<T, Action> MethodReturn<Action> for (T, ) {
    type T = T;

    fn convert(self) -> (Self::T, Vec<Action>) {
        (self.0, vec![])
    }
}

tuple_method_return!(16 P 15 O 14 N 13 M 12 L 11 K 10 J 9 I 8 H 7 G 6 F 5 E 4 D 3 C 2 B 1 A);



#[doc(hidden)]
/// Used by [script] to allow returning multiple sub-Action types from setters.
pub trait SetReturn<Action> : private::Sealed {
    fn convert(self) -> Vec<Action>;
}

macro_rules! tuple_set_return {
    () => { };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        // Handled by MethodReturn
        // impl<$first, $($rest),*> private::Sealed for ($first, $($rest),*) { }

        impl<Action, $first, $($rest),*> SetReturn<Action> for ($first, $($rest),*) 
        where
            $first: Into<Action>,
            $($rest: Into<Action>),*
        {
            fn convert(self) -> Vec<Action> {
                let mut v = vec![
                    self.$n.into(),
                    $(self.$nn.into()),*
                ];
                
                v.reverse();

                v
            }
        }
        tuple_set_return!($($nn $rest)*);
    };
}

tuple_set_return!(15 P 14 O 13 N 12 M 11 L 10 K 9 J 8 I 7 H 6 G 5 F 4 E 3 D 2 C 1 B 0 A);

impl<State, Action, Registry> ScriptableType<State, Action, Registry> for spru::player::Id 
where
    Registry: RegistryTypeEq<State, Action, Self>,
{
    type Type = Self;

    fn register_type(
        registry: &Registry, 
        registration: &mut Registry::RegistrationType<'_, Self::Type>, 
    )
         -> Result<(), Registry::Error>
    {
        registry.register_type_eq(registration, Self::eq)?;
        Ok(())
    }
}
