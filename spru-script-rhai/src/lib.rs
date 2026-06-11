mod context;
pub use context::Context;
mod instance;
pub use instance::Rhai;
pub(crate) mod key;
pub mod marker;
mod output;
pub use output::Output;
mod settings;
pub use settings::Settings;
use spru::item::IdT;

use std::{any, marker::PhantomData, sync::{Arc, RwLock, RwLockWriteGuard}};

type RhaiResult<T> = Result<T, rhai::EvalAltResult>;

pub use spru_script_rhai_macro::scriptable;

use spru_script_base::ScriptablePath;

// Creates function/method/create traits for converting FromDynamic parameters and IntoDynamic return types
// The first number is the max number of parameters when converting parameters (O(2^n) implementations),
// the second number is the max number of parameters when not converting parameters (O(n))
spru_script_rhai_macro::impl_dynamic_fn!(6, 8);


macro_rules! expand_foreach {
    ($dollar:tt { $($pre:tt)* } [$($t:ident),*] { $($post:tt)* }) => {
        macro_rules! _expand_foreach {
            ($dollar t:ident) => {
                $($pre)* $dollar t $($post)*
            };
        }
        $(_expand_foreach!($t);)*
    };
}

expand_foreach!{ $
    { pub trait } [RegisterTypeNoop, RegisterType] { {
        type State: spru::State;
        
        fn register<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) 
        where
            Storage: spru::item::Storage<State = Self::State>,
        {
            
        }
    } }
}

pub trait RegisterMemberNoop {
    type State: spru::State;

    fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        // TODO The type does not give a very good indication of what failed to register,
        // but getting the member name needs some cooperation from the *Wrap types
        tracing::warn!("Could not register member {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterMember<const BITSET: usize> {
    type State: spru::State;

    fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>;
}



#[derive(Debug)]
#[repr(transparent)]
pub struct Wrap<Marker, Args>(Option<Args>, PhantomData<Marker>);

impl<Marker, Args> Wrap<Marker, Args> {
    pub fn new(args: Args) -> Self {
        Self(Some(args), PhantomData)
    }

    pub fn take(&mut self) -> Args {
        self.0.take()
            .expect("take called only once")
    }
}

pub trait IntoDynamic: 'static {
    fn into_dynamic(self) -> rhai::Dynamic;
}

impl<T> IntoDynamic for Option<T> 
where
    T: Clone + Send + Sync + 'static,
{
    fn into_dynamic(self) -> rhai::Dynamic {
        match self {
            Some(some) => rhai::Dynamic::from(some),
            None => rhai::Dynamic::UNIT,
        }
    }
}

impl<T> IntoDynamic for Vec<T> 
where
    T: Clone + Send + Sync + 'static,
{
    fn into_dynamic(self) -> rhai::Dynamic {
        let mut array = rhai::Array::new();
        for e in self {
            array.push(rhai::Dynamic::from(e));
        }
        rhai::Dynamic::from_array(array)
    }
}

pub trait FromDynamic: Sized + 'static {
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>>;
}

impl<T> FromDynamic for Option<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        if dynamic.is_unit() {
            return Ok(None);
        }

        let dynamic = match dynamic.try_cast_result() {
            Ok(some) => return Ok(Some(some)),
            Err(dynamic) => dynamic,
        };

        let dynamic = match dynamic.try_cast_result() {
            Ok(some) => return Ok(some),
            Err(dynamic) => dynamic,
        };

        Err(Box::new(rhai::EvalAltResult::ErrorMismatchDataType(any::type_name::<Option<T>>().to_string(), dynamic.type_name().to_string(), ctx.call_position())))
    }
}

impl<T> FromDynamic for Vec<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
        let dynamic = match dynamic.try_cast_result() {
            Ok(v) => return Ok(v),
            Err(dynamic) => dynamic,
        };

        dynamic.into_typed_array()
            .map_err(|ty| rhai::EvalAltResult::ErrorMismatchDataType(
                any::type_name::<T>().to_string(), 
                ty.to_string(), 
                ctx.call_position(),
            ))
            .map_err(Box::new)
    }
}

expand_foreach! { $
    { impl FromDynamic for } [i8, i16, i32, i64, isize, u8, u16, u32, u64, usize] { {
        fn from_dynamic(ctx: &rhai::NativeCallContext<'_>, dynamic: rhai::Dynamic) -> Result<Self, Box<rhai::EvalAltResult>> {
            let dynamic = match dynamic.try_cast_result() {
                Ok(value) => return Ok(value),
                Err(dynamic) => dynamic,
            };

            let i = dynamic.as_int()
                .map_err(|ty| rhai::EvalAltResult::ErrorMismatchDataType(
                    any::type_name::<Self>().to_string(), 
                    ty.to_string(), 
                    ctx.call_position(),
                ))?;

            i.try_into()
                .map_err(|_| rhai::EvalAltResult::ErrorRuntime(rhai::Dynamic::from(format!("INT value '{i}' too large for {}", any::type_name::<Self>())), ctx.call_position()))
                .map_err(Box::new)
        }
    } }
}

expand_foreach! { $
    { impl IntoDynamic for } [i8, i16, i32, i64, isize, u8, u16, u32, u64, usize] { {
        fn into_dynamic(self) -> rhai::Dynamic {
            // TODO this could panic on u64/usize/isize -> i64 (or u32/i64 if INT is i32)
            rhai::Dynamic::from_int(self as rhai::INT)
        }
    } }
}

macro_rules! wrap_constructors {
    ($($marker:ty => $constructor:ident),* $(,)?) => {
        $(
            impl<Args> Wrap<$marker, Args> {
                pub fn $constructor(args: Args) -> Self {
                    Self::new(args)
                }
            }
        )*
    }
}

wrap_constructors! {
    marker::State => new_state,
    (marker::State, marker::Get) => new_state_get,
    (marker::State, marker::Set) => new_state_set,
    (marker::State, marker::Method) => new_state_method,
    (marker::State, marker::Function) => new_state_function,
    (marker::State, marker::Create) => new_state_create,
    marker::Type => new_type,
    (marker::Type, marker::Get) => new_type_get,
    (marker::Type, marker::Method) => new_type_method,
    (marker::Type, marker::Function) => new_type_function,
    (marker::Type, marker::Eq) => new_type_eq,
}

pub struct Registration1<'r> {
    rhai: &'r mut rhai::Engine,
    globals: rhai::Map,
    statics_maps: rhai::Map,
}

impl<'r> Registration1<'r> {
    pub fn new(rhai: &'r mut rhai::Engine) -> Self {
        Self {
            rhai,
            globals: rhai::Map::new(),
            statics_maps: rhai::Map::new(),
        }
    }

    pub fn type_registration<'r2>(&'r2 mut self, type_path: Option<ScriptablePath>) -> Registration2<'r, 'r2> {
        let statics_map = type_path.map(|tp| (tp, rhai::Map::new()));
        Registration2 {
            registration: self, 
            statics_map,
        }
    }

    pub fn apply(self) -> rhai::Scope<'static> {
        let mut scope = rhai::Scope::new();
        for (base_segment, map) in self.globals {
            scope.push(base_segment, map);
        }
        scope.push(&*crate::key::GLOBAL_TYPE, self.statics_maps);

        scope
    }
}


pub struct Registration2<'r1, 'r2> {
    registration: &'r2 mut Registration1<'r1>,
    statics_map: Option<(ScriptablePath, rhai::Map)>,
}

impl Registration2<'_, '_> {
    pub fn apply(self) {
        if let Some((type_path, statics_map)) = self.statics_map {
            let mut key = String::new();
            write_scriptable_path(&mut key, &type_path);
            self.registration.statics_maps.insert(key.into(), statics_map.into());
        }
    }
}

pub trait Register {
    fn register(engine: &mut rhai::Engine);
}

pub type TypeArgs<Action, T> = (PhantomData<(Action, T)>, );
pub type TypeWrap<Action, T> = Wrap<marker::Type, TypeArgs<Action, T>>;

impl<Action, T> RegisterTypeNoop for TypeWrap<Action, T> 
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, T> RegisterType for &mut TypeWrap<Action, T>
where 
    Action: spru::Action,
    T: Clone + Send + Sync + 'static,
{
    type State = Action::State;

    fn register<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        let (_, ) = self.take();
        registration.registration.rhai.register_type::<T>();

        registration.registration.rhai.register_fn("flatten", flatten::<T>);
        
        registration.registration.rhai.register_fn("to_array", to_array::<T>);

        if let Some((_, statics_map)) = &mut registration.statics_map {
            statics_map.insert("none".into(), rhai::Dynamic::from(None::<T>));

            #[allow(deprecated)]
            let from_array = rhai::FnPtr::from_fn("from_array", from_array::<T>)
                .expect("function name must be valid");

            statics_map.insert(
                "from_array".into(), 
                from_array.into(),
            );
        }
    }
}

pub type TypeEqArgs<Action, T> = (PhantomData<(Action, T)>, );
pub type TypeEqWrap<Action, T> = Wrap<(marker::Type, marker::Eq), TypeEqArgs<Action, T>>;

impl<Action, T> RegisterMemberNoop for TypeEqWrap<Action, T>
where 
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, T> RegisterMember<0> for &mut TypeEqWrap<Action, T>
where
    Action: spru::Action,
    T: PartialEq + Clone + Sync + Send + 'static,
{
    type State = Action::State;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        let (_, ) = self.take();
        registration.registration.rhai.register_fn("==", move |t: &mut T, t2: T| t == &t2);
        registration.registration.rhai.register_fn("!=", move |t: &mut T, t2: T| t != &t2);
    }
}


pub type TypeGetArgs<'n, Action, T, U> = (&'n str, fn(&T) -> U, PhantomData<Action>);
pub type TypeGetWrap<'n, Action, T, U> = Wrap<(marker::Type, marker::Get), TypeGetArgs<'n, Action, T, U>>;

impl<Action, T, U> RegisterMemberNoop for TypeGetWrap<'_, Action, T, U>
where 
    Action: spru::Action,
{ 
    type State = Action::State;
}


pub type TypeMethodArgs<'n, Action, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<Action>);
pub type TypeMethodWrap<'n, Action, T, Args, Ret> = Wrap<(marker::Type, marker::Method), TypeMethodArgs<'n, Action, T, Args, Ret>>;

impl<Action, T, Args, Ret> RegisterMemberNoop for TypeMethodWrap<'_, Action, T, Args, Ret> 
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

pub type TypeFunctionArgs<'n, Action, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<Action>);
pub type TypeFunctionWrap<'n, Action, Args, Ret> = Wrap<(marker::Type, marker::Function), TypeFunctionArgs<'n, Action, Args, Ret>>;

impl<Action, Args, Ret> RegisterMemberNoop for TypeFunctionWrap<'_, Action, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

pub type StateArgs<Action, T> = (PhantomData<(Action, T)>, );
pub type StateWrap<Action, T> = Wrap<marker::State, StateArgs<Action, T>>;

impl<Action, T> RegisterTypeNoop for StateWrap<Action, T> 
where
    Action: spru::Action,
{
    type State = Action::State;
}

impl<Action, T> RegisterType for &mut StateWrap<Action, T>
where 
    Action: spru::Action,
    T: spru::item::storage::Storable<Action::State> + Clone + Send + Sync + 'static,
{
    type State = Action::State;

    fn register<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        let (_, ) = self.take();
        registration.registration.rhai.register_type::<IdT<T>>();
        registration.registration.rhai.register_fn("==", |idt: &mut IdT<T>, idt2: IdT<T>| *idt == idt2);
        registration.registration.rhai.register_fn("!=", |idt: &mut IdT<T>, idt2: IdT<T>| *idt != idt2);

        registration.registration.rhai.register_fn("flatten", flatten::<IdT<T>>);

        registration.registration.rhai.register_fn("exists", |ctx: rhai::NativeCallContext<'_>, idt: &mut IdT<T>| {
            let mut handle = LedgerHandle::from_rhai(&ctx);
            let ledger = unsafe { handle.get_mut::<Storage, Action>() };
            ledger.get(*idt).is_ok()
        });
        registration.registration.rhai.register_fn("to_array", to_array::<IdT<T>>);

        if let Some((_, statics_map)) = &mut registration.statics_map {
            statics_map.insert("none".into(), rhai::Dynamic::from(None::<IdT<T>>));
            
            #[allow(deprecated)]
            let from_array = rhai::FnPtr::from_fn("from_array", from_array::<IdT<T>>)
                .expect("function name must be valid");

            statics_map.insert(
                "from_array".into(), 
                from_array.into(),
            );
        }
    }
}

pub type StateGetArgs<'n, Action, T, U> = (&'n str, fn(&T) -> U, PhantomData<Action>);
pub type StateGetWrap<'n, Action, T, U> = Wrap<(marker::State, marker::Get), StateGetArgs<'n, Action, T, U>>;

impl<Action, T, U> RegisterMemberNoop for StateGetWrap<'_, Action, T, U>
where
    Action: spru::Action,
{ 
    type State = Action::State;
} 

pub type StateSetArgs<'n, Action, T, U, Ret> = (&'n str, fn(&T, U) -> Ret, PhantomData<Action>);
pub type StateSetWrap<'n, Action, T, U, Ret> = Wrap<(marker::State, marker::Set), StateSetArgs<'n, Action, T, U, Ret>>;

impl<Action, T, U, Ret> RegisterMemberNoop for StateSetWrap<'_, Action, T, U, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
    
    fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_,'_>)
    where 
        Storage: spru::item::Storage<State = Self::State>,
    {
        
    }
    
}

pub type StateMethodArgs<'n, Action, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<Action>);
pub type StateMethodWrap<'n, Action, T, Args, Ret> = Wrap<(marker::State, marker::Method), StateMethodArgs<'n, Action, T, Args, Ret>>;

impl<Action, T, Args, Ret> RegisterMemberNoop for StateMethodWrap<'_, Action, T, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

pub type StateFunctionArgs<'n, Action, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<Action>);
pub type StateFunctionWrap<'n, Action, Args, Ret> = Wrap<(marker::State, marker::Function), StateFunctionArgs<'n, Action, Args, Ret>>;

impl<Action, Args, Ret> RegisterMemberNoop for StateFunctionWrap<'_, Action, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

pub type StateCreateArgs<'n, Action, T, Args, Create> = (&'n str, fn(Args) -> Create, PhantomData<(Action, T)>);
pub type StateCreateWrap<'n, Action, T, Args, Create> = Wrap<(marker::State, marker::Create), StateCreateArgs<'n, Action, T, Args, Create>>;

impl<Action, T, Args, Create> RegisterMemberNoop for StateCreateWrap<'_, Action, T, Args, Create>
where
    Action: spru::Action
{ 
    type State = Action::State;
}

// https://rhai.rs/book/patterns/references.html
#[derive(Clone)]
struct LedgerHandle {
    pointer: Arc<RwLock<*mut ()>>,
}

// SAFETY: LedgerHandle locks all pointer access, the only concern is LedgerHandle does not outlive the original Ledger reference.
unsafe impl Send for LedgerHandle {}
unsafe impl Sync for LedgerHandle {}

impl LedgerHandle {
    pub fn new<'l, Storage, Action>(ledger: &mut spru::interactor::Ledger<'l, Storage, Action>) -> Self {
        let pointer = ledger as *mut spru::interactor::Ledger<'l, Storage, Action>;
        Self {
            pointer: Arc::new(RwLock::new(pointer.cast())),
        }
    }

    pub unsafe fn get_mut<'l, 'i, Storage, Action>(&'i mut self) 
        -> LedgerMut<'l, 'i, Storage, Action>
    {
        let guard = self.pointer.write()
            .expect("Ledger lock poisoned");
        
        let ledger = guard.cast::<spru::interactor::Ledger<'l, Storage, Action>>();
        let ledger = unsafe { &mut *ledger };
        LedgerMut { _guard: guard, ledger }
    }

    pub fn from_rhai(ctx: &rhai::NativeCallContext) -> Self {
        let handle = ctx.tag()
            .expect("Ledger handle not set")
            .clone();
        
        handle.try_cast_result::<crate::LedgerHandle>()
            .expect("Expected Ledger handle")
    }
}

struct LedgerMut<'l, 'i, Storage, Action> {
    _guard: RwLockWriteGuard<'i, *mut ()>,
    ledger: &'i mut spru::interactor::Ledger<'l, Storage, Action>, 
}

impl<'l, 'i, Storage, Action> std::ops::Deref for LedgerMut<'l, 'i, Storage, Action> {
    type Target = spru::interactor::Ledger<'l, Storage, Action>;

    fn deref(&self) -> &Self::Target {
        self.ledger
    }
}

impl<'l, 'i, Storage, Action> std::ops::DerefMut for LedgerMut<'l, 'i, Storage, Action> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ledger
    }
}

fn pop_type<T>(ctx: &rhai::NativeCallContext<'_>, args: &mut &mut [&mut rhai::Dynamic]) -> Result<T, rhai::EvalAltResult> 
where
    T: Clone + Sync + Send + 'static,
{
    if let Some(arg) = args.split_off_first_mut() {
        let arg = arg.take();
        match arg.try_cast_result::<T>() {
            Ok(arg) => {
                Ok(arg)
            }
            Err(arg) => {
                let expected = std::any::type_name::<T>().to_string();
                let actual = arg.type_name().to_string();
                Err(rhai::EvalAltResult::ErrorMismatchDataType(expected, actual, ctx.call_position()))
            }
        }
    } else {
        Err(rhai::EvalAltResult::ErrorRuntime("Not enough arguments".into(), ctx.call_position()))
    }
}

fn extra_args_error(ctx: &rhai::NativeCallContext<'_>, args: &[&mut rhai::Dynamic]) -> RhaiResult<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(rhai::EvalAltResult::ErrorRuntime("Too many arguments".into(), ctx.call_position()))
    }
}

fn from_array<T>(ctx: rhai::NativeCallContext<'_>, mut args: &mut [&mut rhai::Dynamic]) 
    -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> 
where
    T: Clone + Send + Sync + 'static,
{
    // This will be called from a map, so ignore the `this` arguments
    let _ = args.split_off_first_mut();
    let array = pop_type::<rhai::Array>(&ctx, &mut args)?;
    extra_args_error(&ctx, &mut args)?;

    let mut v = Vec::with_capacity(array.len());
    for element in array {
        match element.try_cast_result::<T>() {
            Ok(t) => {
                v.push(t);
            }
            Err(element) => {
                return Err(Box::new(rhai::EvalAltResult::ErrorMismatchDataType(
                    std::any::type_name::<T>().to_string(), 
                    element.type_name().to_string(), 
                    ctx.call_position()
                )));
            }
        }
    }
    Ok(rhai::Dynamic::from(v))
}

fn flatten<T>(option: &mut Option<T>) -> rhai::Dynamic 
where 
    T: Clone + Send + Sync + 'static,
{
    match option {
        Some(some) => rhai::Dynamic::from(some.clone()),
        None => rhai::Dynamic::UNIT,
    }
}

fn to_array<T>(v: &mut Vec<T>) -> rhai::Array 
where
    T: Clone + Send + Sync + 'static,
{
    v.iter()
        .cloned()
        .map(rhai::Dynamic::from)
        .collect::<rhai::Array>()
}

fn write_scriptable_path(s: &mut String, type_path: &ScriptablePath) {
    use std::fmt::Write as _;

    let &ScriptablePath(path, type_args) = type_path;

    if let Some((first, rest)) = path.split_first() {
        write!(s, "{first}").unwrap();
        for segment in rest {
            write!(s, "::{segment}").unwrap();
        }

        if let Some((first, rest)) = type_args.split_first() {
            write!(s, "<").unwrap();
            write_scriptable_path(s, first);
            for arg in rest {
                write!(s, ",").unwrap();
                write_scriptable_path(s, arg);
            }
            write!(s, ">").unwrap();
        }
    }
}
