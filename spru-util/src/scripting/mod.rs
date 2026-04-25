//! Traits for creating scripting implementations.

#[cfg(feature = "lua")]
pub mod lua;

use tagset::tagset_meta;
use telety::telety;

/// A [spru::State] with scripting support. Implement using the [tagset::tagset] macro.
/// ```ignore
/// #[tagset(impl<Action, Registry> ScriptableState<Action, Registry>)]
/// ```
#[telety(crate::scripting, alias_traits = "always")]
#[tagset_meta]
#[meta(bounds(
    Registry: self::Registry<Self, Action>,
    for<VAR> Registry: self::RegistryType<Self, Action, VAR>,
))]
pub trait ScriptableState<Action, Registry: ?Sized>: spru::State 
where
    Registry: self::Registry<Self, Action>,
{
    /// Registers substates with the scripting implementation.
    /// [ScriptRegistryType::register_type] should be called for each applicable type.
    #[meta(default {
        foreach!(VAR => {
            scripting::RegistryType::<Self, Action, VAR>::register_type::<Storage>(registry, registration)?;
        });
        Ok(())
    })]
    fn register<Storage>(registry: &Registry, registration: &mut Registry::TypeRegistration<'_, Storage>) 
        -> Result<(), Registry::Error>
    where
        Storage: spru::item::Storage<State = Self>,
    ;
}


pub trait ScriptableType<State, Action, Registry: ?Sized>: Sized + 'static
where
    Registry: self::Registry<State, Action>,
{
    /// Registers members of this type with the scripting implementation.
    /// [ScriptRegistryGetter::register_get], etc. should be called for each field/method.
    fn register<Storage>(registry: &Registry, registration: &mut Registry::MemberRegistration<'_, Storage, Self>)
         -> Result<(), Registry::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// A scripting implementation. Implementations will also implement [ScriptRegistryType] and [ScriptRegistryGetter], etc. based
/// on the specific types they can support. 
pub trait Registry<State, Action> {
    /// The mutable type responsible for performing type registration based on the concrete Storage implementation.
    type TypeRegistration<'r, Storage>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    /// The mutable type responsible for performing field/method registration based on the concrete Storage implementation.
    type MemberRegistration<'r, Storage, T: 'static>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    /// The error type for the scriping implementation.
    type Error;
}

/// Scripting implementations implement this for types they support.
pub trait RegistryType<State, Action, T>: Registry<State, Action> {
    /// Register a type and all its fields/methods with the scripting implementation.
    fn register_type<Storage>(&self, registration: &mut Self::TypeRegistration<'_, Storage>) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as field getters.
pub trait RegistryGetter<State, Action, T, U>: Registry<State, Action> {
    /// Register a read-only getter for a field.
    fn register_get<Storage>(&self, registration: &mut Self::MemberRegistration<'_, Storage, T>, ident: &str, getter: fn(&T) -> U)
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as field setters.
pub trait RegistrySetter<State, Action, T, U>: Registry<State, Action> {
    /// Register a read-only setter for a field.
    fn register_set<Storage>(&self, registration: &mut Self::MemberRegistration<'_, Storage, T>, ident: &str, setter: fn(&T, U) -> Vec<Action>) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

// pub trait ScriptableMethod<Args, Ret>: ScriptTypeRegistry {
//     fn register_method(&mut self, ident: &str, method: impl Fn(&Self::T, Args) -> (Vec<Self::Action>, Ret) + 'static) -> Result<(), Self::Error>;
// }

