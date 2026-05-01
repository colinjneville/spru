//! Language-agnostic support for scripting in spru. 
//! Game Items should use the included derive macros or implement the `Scriptable*` traits manually, 
//! and specific scripting implementations should implement the `Registry*` and `Registration*` traits.
mod interaction;


use tagset::tagset_meta;
use telety::telety;

#[derive(Debug)]
pub struct ScriptablePath(pub &'static [&'static str], pub &'static [Self]);

/// Parses a rust type path during macro expansion. Used for implementing [ScriptableState::register].
/// Not normally needed for manual use.
pub use spru_script_macro::scriptable_path;

// const S: ScriptablePath = ScriptablePath(&["sadf", "ewrt", "cxvb"], &[ScriptablePath(&["i32"], &[])]);

/// A [spru::State] with scripting support. Implement using the [tagset::tagset] macro.
/// ```ignore
/// #[tagset(impl<Action, Registry> ScriptableState<Action, Registry>)]
/// ```
#[telety(crate, alias_traits = "always")]
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
            spru_script::RegistryType::<Self, Action, VAR>::register_type::<Storage>(
                registry, 
                registration, 
                Some(spru_script::scriptable_path!(VAR))
            )?;
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
    fn register<Storage>(
        registry: &Registry, 
        registration: &mut Registry::MemberRegistration<'_, Storage, Self>, 
    )
         -> Result<(), Registry::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

pub trait LanguageNoRoot<State, Action, Return> {
    type Registry: Registry<State, Action>;
    type Error;
    
    /// Execute a script without access to the Game's Root. 
    /// Mainly useful for the Game Init, where no root exists.
    fn exec_no_root<Storage, Context, Output>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> Result<Return, Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
        State: spru::State + ScriptableState<Action, Self::Registry>,
        Action: spru::Action<State = State>,
    ;
}

pub trait Language<State, Action, Return, Root>: LanguageNoRoot<State, Action, Return> {
    /// Execute a script with access to the Game's Root. 
    fn exec<Storage, Context, Output>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> Result<Return, Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
        State: spru::State + ScriptableState<Action, Self::Registry>,
        Action: spru::Action<State = State>,
        Context: spru::interactor::GetRoot<Root = Root>,
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
    fn register_type<Storage>(
        &self, 
        registration: &mut Self::TypeRegistration<'_, Storage>,
        type_path: Option<ScriptablePath>,
    ) 
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
    /// Register a setter for a field.
    fn register_set<Storage>(&self, registration: &mut Self::MemberRegistration<'_, Storage, T>, ident: &str, setter: fn(&T, U) -> Vec<Action>) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}

/// Scripting implementations implement this for types they support as methods.
pub trait RegistryMethod<State, Action, T, Args, Ret>: Registry<State, Action> {
    /// Register a method.
    fn register_method<Storage>(&self, registration: &mut Self::MemberRegistration<'_, Storage, T>, ident: &str, setter: fn(&T, Args) -> (Ret, Vec<Action>)) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}


/// Scripting implementations implement this for types they support as constructors.
pub trait RegistryCreate<State, Action, Create, T, Args>: Registry<State, Action> 
where
    Create: spru::action::Create<T = T> + Into<Action>,
{
    /// Register a constructor.
    fn register_create<Storage>(&self, registration: &mut Self::MemberRegistration<'_, Storage, T>, ident: &str, setter: fn(Args) -> Create) 
        -> Result<(), Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
    ;
}