mod context;
pub use context::Context;
mod instance;
pub use instance::{Rhai, RhaiInstance};
pub(crate) mod key;
mod output;
pub use output::Output;
mod settings;
pub use settings::Settings;
use spru::item::IdT;

use std::{any, sync::{Arc, RwLock, RwLockWriteGuard}};

type RhaiResult<T> = Result<T, rhai::EvalAltResult>;

use spru_script::wrap::*;
use spru_script::ScriptablePath;

#[macro_export]
macro_rules! _rhai {
    (<$storage:ty, $action:ty> $registration:ident {
        $(
            $macro_path:path => $ty:path $(as $type_alias:path)?;
        )*
    } ) => {
        #[allow(unused_imports)]
        use spru_script_rhai::{
            RegisterStateMemberNoop as _, RegisterStateMember as _,
            RegisterStateNoop as _, RegisterState as _, 
        };

        $(
            $macro_path!(<$storage, $action> $registration => $ty $(as $type_alias)?);
        )*
    };
    ($registration:ident {
        $(
            $macro_path:path => $ty:path $(as $type_alias:path)?;
        )*
    } ) => {
        #[allow(unused_imports)]
        use spru_script_rhai::{
            RegisterStatelessMemberNoop as _, RegisterStatelessMember as _,
            RegisterStatelessNoop as _, RegisterStateless as _, 
        };

        $(
            $macro_path!($registration => $ty $(as $type_alias)?);
        )*
    };
}
pub use _rhai as rhai;

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

pub trait RegisterStatelessNoop {
    fn register_stateless(&mut self, _registration: &mut Registration2<'_, '_>) {
        tracing::warn!("Could not register stateless {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStateless {
    fn register_stateless(&mut self, _registration: &mut Registration2<'_, '_>);
}

pub trait RegisterStateNoop {
    type State: spru::State;
    
    fn register_state<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        tracing::warn!("Could not register state {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterState {
    type State: spru::State;
    
    fn register_state<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    ;
}

pub trait RegisterStatelessMemberNoop {
    fn register_stateless_member(&mut self, _registration: &mut Registration2<'_, '_>, ) {
        // TODO The type does not give a very good indication of what failed to register,
        // but getting the member name needs some cooperation from the *Wrap types
        tracing::warn!("Could not register member {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStateMemberNoop {
    type State: spru::State;

    fn register_state_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>,
    {
        // TODO The type does not give a very good indication of what failed to register,
        // but getting the member name needs some cooperation from the *Wrap types
        tracing::warn!("Could not register member {ty}", ty = any::type_name::<Self>());
    }
}

pub trait RegisterStatelessMember<const BITSET: usize> {
    fn register_stateless_member(&mut self, _registration: &mut Registration2<'_, '_>, );
}

pub trait RegisterStateMember<const BITSET: usize> {
    type State: spru::State;

    fn register_state_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>, ) 
    where
        Storage: spru::item::Storage<State = Self::State>;
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

impl<T> RegisterStatelessNoop for StatelessWrap<T> { }

impl<T> RegisterStateless for &mut StatelessWrap<T>
where 
    T: Clone + Send + Sync + 'static,
{
    fn register_stateless(&mut self, registration: &mut Registration2<'_, '_>) {
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



impl<T> RegisterStatelessMemberNoop for StatelessEqWrap<T> { }

impl<T> RegisterStatelessMember<0> for &mut StatelessEqWrap<T>
where
    T: PartialEq + Clone + Sync + Send + 'static,
{
    fn register_stateless_member(&mut self, registration: &mut Registration2<'_, '_>) {
        let (_, ) = self.take();
        registration.registration.rhai.register_fn("==", move |t: &mut T, t2: T| t == &t2);
        registration.registration.rhai.register_fn("!=", move |t: &mut T, t2: T| t != &t2);
    }
}




impl<T, U> RegisterStatelessMemberNoop for StatelessGetWrap<'_, T, U> { }

impl<T, Args, Ret> RegisterStatelessMemberNoop for StatelessMethodWrap<'_, T, Args, Ret> { }


impl<Args, Ret> RegisterStatelessMemberNoop for StatelessFunctionWrap<'_, Args, Ret> { }

impl<Action, T> RegisterStateNoop for StateWrap<Action, T> 
where
    Action: spru::Action,
{
    type State = Action::State;
}

impl<Action, T> RegisterState for &mut StateWrap<Action, T>
where 
    Action: spru::Action,
    T: spru::item::storage::Storable<Action::State> + Clone + Send + Sync + 'static,
{
    type State = Action::State;

    fn register_state<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
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

impl<Action, T, U> RegisterStateMemberNoop for StateGetWrap<'_, Action, T, U>
where
    Action: spru::Action,
{ 
    type State = Action::State;
} 

impl<Action, T, U, Ret> RegisterStateMemberNoop for StateSetWrap<'_, Action, T, U, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
    
    fn register_state_member<Storage>(&mut self, _registration: &mut Registration2<'_,'_>)
    where 
        Storage: spru::item::Storage<State = Self::State>,
    {
        
    }
    
}

impl<Action, T, Args, Ret> RegisterStateMemberNoop for StateMethodWrap<'_, Action, T, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, Args, Ret> RegisterStateMemberNoop for StateFunctionWrap<'_, Action, Args, Ret>
where
    Action: spru::Action,
{ 
    type State = Action::State;
}

impl<Action, T, Args, Create> RegisterStateMemberNoop for StateCreateWrap<'_, Action, T, Args, Create>
where
    Action: spru::Action
{ 
    type State = Action::State;
}

// https://rhai.rs/book/patterns/references.html
#[derive(Clone)]
enum LedgerHandle {
    StorageOnly(Arc<*const ()>),
    Ledger(Arc<RwLock<*mut ()>>),
}

// SAFETY: LedgerHandle locks all pointer access, the only concern is LedgerHandle does not outlive the original Ledger reference.
unsafe impl Send for LedgerHandle {}
unsafe impl Sync for LedgerHandle {}

impl LedgerHandle {
    pub fn new<'l, Storage, Action>(ledger: &mut spru::interactor::Ledger<'l, Storage, Action>) -> Self {
        let pointer = ledger as *mut spru::interactor::Ledger<'l, Storage, Action>;
        Self::Ledger(Arc::new(RwLock::new(pointer.cast())))
    }

    pub fn new_readonly<'l, Storage>(storage: &Storage) -> Self {
        let pointer = storage as *const Storage;
        Self::StorageOnly(Arc::new(pointer.cast()))
    }

    // pub unsafe fn get<'i, Storage, Action: 'i>(&'i self)
    //     -> StorageRef<'i, Storage>
    // {
    //     match self {
    //         LedgerHandle::StorageOnly(pointer) => {
    //             let storage = pointer.cast::<Storage>();
    //             let storage = unsafe { &*storage };
    //             StorageRef { _guard: None, storage, }
    //         },
    //         LedgerHandle::Ledger(pointer) => {
    //             let guard = pointer.read()
    //                 .expect("Ledger lock poisoned");
    //             let ledger = guard.cast::<spru::interactor::Ledger<'_, Storage, Action>>();
    //             let ledger = unsafe { &mut *ledger };
    //             StorageRef { _guard: Some(guard), storage: ledger.storage() }
    //         },
    //     }
    // }

    pub unsafe fn get_mut<'l, 'i, Storage, Action>(&'i mut self) 
        -> LedgerMut<'l, 'i, Storage, Action>
    {
        match self {
            LedgerHandle::StorageOnly(pointer) => {
                let storage = pointer.cast::<Storage>();
                let storage = unsafe { &*storage };
                
                LedgerMut::Storage { storage }
            },
            LedgerHandle::Ledger(pointer) => {
                let guard = pointer.write()
                    .expect("Ledger lock poisoned");
                
                let ledger = guard.cast::<spru::interactor::Ledger<'l, Storage, Action>>();
                let ledger = unsafe { &mut *ledger };
                LedgerMut::Ledger { _guard: guard, ledger }
            },
        }
        
    }

    pub fn from_rhai(ctx: &rhai::NativeCallContext) -> Self {
        let handle = ctx.tag()
            .expect("Ledger handle not set")
            .clone();
        
        handle.try_cast_result::<crate::LedgerHandle>()
            .expect("Expected Ledger handle")
    }
}

// struct StorageRef<'i, Storage> {
//     _guard: Option<RwLockReadGuard<'i, *mut ()>>,
//     storage: &'i Storage, 
// }

// impl<'i, Storage> std::ops::Deref for StorageRef<'i, Storage> {
//     type Target = Storage;

//     fn deref(&self) -> &Self::Target {
//         self.storage
//     }
// }

enum LedgerMut<'l, 'i, Storage, Action> {
    Storage {
        storage: &'i Storage,
    },
    Ledger {
        _guard: RwLockWriteGuard<'i, *mut ()>,
        ledger: &'i mut spru::interactor::Ledger<'l, Storage, Action>, 
    },
}

impl<'l, 'i, Storage, Action> LedgerMut<'l, 'i, Storage, Action> {
    fn get<T>(&self, id: IdT<T>) 
        -> Result<&T, spru::item::storage::Error> 
    where
        Storage: spru::item::Storage,
        T: spru::item::storage::Storable<Storage::State>,
    {
        match self {
            LedgerMut::Storage { storage } => storage.get(id)
                .map(spru::Item::get),
            LedgerMut::Ledger { _guard, ledger } => ledger.get(id)
                .map(|existing| existing.state()),
        }
    }

    fn ledger(&mut self) -> Result<&mut spru::interactor::Ledger<'l, Storage, Action>, &'static str> {
        match self {
            LedgerMut::Storage { .. } => Err("Attempted to modify state during read-only script evaluation"),
            LedgerMut::Ledger { _guard, ledger } => Ok(ledger),
        }
    }
}

// impl<'l, 'i, Storage, Action> std::ops::Deref for LedgerMut<'l, 'i, Storage, Action> {
//     type Target = spru::interactor::Ledger<'l, Storage, Action>;

//     fn deref(&self) -> &Self::Target {
//         self.ledger
//     }
// }

// impl<'l, 'i, Storage, Action> std::ops::DerefMut for LedgerMut<'l, 'i, Storage, Action> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.ledger
//     }
// }

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
