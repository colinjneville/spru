mod context;
pub use context::Context;
pub(crate) mod func;
mod instance;
pub use instance::Rhai;
pub(crate) mod key;
pub mod marker;
mod output;
pub use output::Output;
mod settings;
pub use settings::Settings;
mod registration;
pub use registration::{Registration, RegistrationState, RegistrationType};
mod registry;
pub use registry::Registry;
use spru::item::IdT;

use std::{cell::RefCell, marker::PhantomData, sync::{Arc, RwLock, RwLockWriteGuard, atomic::{self, AtomicI64}}};

type RhaiResult<T> = Result<T, rhai::EvalAltResult>;

pub use spru_script_rhai_macro::{postlude, prelude, scriptable};

#[tagset::tagset_meta]
pub trait Lexicon {
    #[meta(default {
        ::spru_script::prelude!(rhai);
        foreach!(VAR => {
            VAR!([VAR]);
        });
        ::spru_script::postlude!();
    })]
    fn register<Storage, Action>(rhai: &mut rhai::Engine);
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

macro_rules! wrap_constructors {
    ($($marker:path => $constructor:ident),* $(,)?) => {
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
    marker::Type => new_type,
    marker::Get => new_get,
    marker::Set => new_set,
    marker::Method => new_method,
    marker::Function => new_function,
    marker::Create => new_create,
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

    pub fn type_registration<'r2>(&'r2 mut self, type_path: Option<spru_script::ScriptablePath>) -> Registration2<'r, 'r2> {
        let statics_map = type_path.map(|tp| (tp, rhai::Map::new()));
        Registration2 {
            registration: self, 
            statics_map,
        }
    }

    pub fn apply(self) {
        let mut scope = rhai::Scope::new();
        for (base_segment, map) in self.globals {
            scope.push(base_segment, map);
        }
        scope.push(&*crate::key::GLOBAL_TYPE, self.statics_maps);
    }
}


pub struct Registration2<'r1, 'r2> {
    registration: &'r2 mut Registration1<'r1>,
    statics_map: Option<(spru_script::ScriptablePath, rhai::Map)>,
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
    fn register(engine: &mut rhai::Engine) -> RhaiResult<()>;
}

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
    { pub trait } [RegisterTypeNoop, RegisterTypeStd] { {
        fn register(&mut self, _registration: &mut Registration1<'_>) -> RhaiResult<()> {
            Ok(())
        }
    } }
}

type TypeArgs<T> = (PhantomData<T>, );

impl<T> RegisterTypeNoop for Wrap<marker::Type, TypeArgs<T>> { }

impl<T> RegisterTypeStd for &mut Wrap<marker::Type, TypeArgs<T>>
where 
    T: Clone + Send + Sync + 'static,
{
    fn register(&mut self, registration: &mut Registration1<'_>) -> RhaiResult<()> {
        let (_, ) = self.take();
        registration.rhai.register_type::<T>();
        Ok(())
    }
}

expand_foreach! { $
    { pub trait } [RegisterTypeGetNoop, RegisterTypeGetStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type TypeGetArgs<'n, Action, T, U> = (&'n str, fn(&T) -> U, PhantomData<Action>);

impl<Action, T, U> RegisterTypeGetStd for Wrap<(marker::Type, marker::Get), TypeGetArgs<'_, Action, T, U>> 
where 
    Action: spru::Action,
{ 
    type Action = Action;
}

impl<Action, T, U> RegisterTypeGetStd for &mut Wrap<(marker::Type, marker::Get), TypeGetArgs<'_, Action, T, U>> 
where
    Action: spru::Action,
    T: Clone + Sync + Send + 'static,
    U: Clone + Sync + Send + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, get, _) = self.take();
        registration.registration.rhai.register_get(name, move |t: &mut T| get(t));
        Ok(())
    }
}

expand_foreach! {$
    { pub trait } [RegisterTypeMethodNoop, RegisterTypeMethodStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type TypeMethodArgs<'n, Action, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<Action>);

impl<Action, T, Args, Ret> RegisterTypeMethodNoop for Wrap<(marker::Type, marker::Method), TypeMethodArgs<'_, Action, T, Args, Ret>> 
where
    Action: spru::Action,
{ 
    type Action = Action;
}

impl<Action, T, Args, Ret> RegisterTypeMethodStd for &mut Wrap<(marker::Type, marker::Method), TypeMethodArgs<'_, Action, T, Args, Ret>> 
where
    Action: spru::Action,
    T: Clone + Send + Sync + 'static,
    Args: RegisterUnpacked + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, method, _) = self.take();
        let reg = rhai::FuncRegistration::new(name);
        Args::register_unpacked(registration.registration.rhai, reg, move |_ctx: rhai::NativeCallContext<'_>, this: &mut T, args: Args| {
            let ret = method(this, args);

            Ok(ret)
        });
            
        Ok(())
    }
}

expand_foreach! {$
    { pub trait } [RegisterTypeFunctionNoop, RegisterTypeFunctionStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>, ) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type TypeFunctionArgs<'n, Action, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<Action>);

impl<Action, Args, Ret> RegisterTypeFunctionNoop for Wrap<(marker::Type, marker::Function), TypeFunctionArgs<'_, Action, Args, Ret>> 
where
    Action: spru::Action,
{ 
    type Action = Action;
}

impl<Action, Args, Ret> RegisterTypeFunctionStd for &mut Wrap<(marker::Type, marker::Function), TypeFunctionArgs<'_, Action, Args, Ret>> 
where
    Action: spru::Action,
    Args: FromArguments + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    type Action = Action;
    
    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, function, _) = self.take();
        if let Some((_, statics_map)) = registration.statics_map.as_mut() {
            #[allow(deprecated)]
            let fn_ptr = rhai::FnPtr::from_fn(name, move |ctx, mut args| {
                // Self parameter is unused
                args.split_off_first_mut();
                let args = Args::from_arguments(&ctx, args)?;
                Ok(rhai::Dynamic::from(function(args)))
            }).map_err(|e| *e)?;

            statics_map.insert(name.into(), fn_ptr.into());
        }

        Ok(())
    }
}

expand_foreach!{ $
    { pub trait } [RegisterStateNoop, RegisterStateStd] { {
        fn register(&mut self, _registration: &mut Registration1<'_>) -> RhaiResult<()> {
            Ok(())
        }
    } }
}

type StateArgs<T> = (PhantomData<T>, );

impl<T> RegisterStateNoop for Wrap<marker::State, StateArgs<T>> { }

impl<T> RegisterStateStd for &mut Wrap<marker::State, StateArgs<T>>
where 
    T: Clone + Send + Sync + 'static,
{
    fn register(&mut self, registration: &mut Registration1<'_>) -> RhaiResult<()> {
        let (_, ) = self.take();
        registration.rhai.register_type::<IdT<T>>();
        Ok(())
    }
}

expand_foreach! { $
    { pub trait } [RegisterStateGetNoop, RegisterStateGetStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) 
            -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type StateGetArgs<'n, Action, T, U> = (&'n str, fn(&T) -> U, PhantomData<Action>);

impl<Action, T, U> RegisterStateGetNoop for Wrap<(marker::State, marker::Get), StateGetArgs<'_, Action, T, U>> 
where
    Action: spru::Action,
{ 
    type Action = Action;
} 

impl<Action, T, U> RegisterStateGetStd for &mut Wrap<(marker::State, marker::Get), StateGetArgs<'_, Action, T, U>> 
where
    Action: spru::Action,
    T: spru::item::storage::Storable<Action::State>,
    U: Clone + Sync + Send + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
        -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, get, _) = self.take();
        let f = move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>| -> Result<U, Box<rhai::EvalAltResult>> {
            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            Ok(get(&*item))
        };

        // This roundabout registration is necessary because rhai::Engine::register_get's trait bounds includes
        // a type rhai doesn't publically expose.
        rhai::FuncRegistration::new_getter(name)
            .with_volatility(true)
            .register_into_engine(registration.registration.rhai, f);
        
        Ok(())
    }
}

expand_foreach! { $
    { pub trait } [RegisterStateSetNoop, RegisterStateSetStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) 
            -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type StateSetArgs<'n, Action, T, U> = (&'n str, fn(&T, U) -> Vec<Action>);

impl<Action, T, U> RegisterStateSetNoop for Wrap<(marker::State, marker::Set), StateSetArgs<'_, Action, T, U>> 
where
    Action: spru::Action<State: spru::State>,
{ 
    type Action = Action;
}

impl<Action, T, U> RegisterStateSetStd for &mut Wrap<(marker::State, marker::Set), StateSetArgs<'_, Action, T, U>> 
where
    Action: spru::Action<State: spru::State>,
    T: spru::item::storage::Storable<Action::State>,
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) 
        -> RhaiResult<()>
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, set) = self.take();
        let f = move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>, value: U| -> Result<(), Box<rhai::EvalAltResult>> {
            let mut handle = crate::LedgerHandle::from_rhai(&ctx);
            let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            let actions = set(&*item, value);

            for action in actions {
                ledger.enqueue_action(this.untyped(), action);
            }
            ledger.flush()
                .map_err(|e| format!("Failed to flush actions: {e}"))?;

            Ok(())
        };

        rhai::FuncRegistration::new_setter(name)
            .with_purity(false)
            .register_into_engine(registration.registration.rhai, f);
        Ok(())
    }
}

expand_foreach! {$
    { pub trait } [RegisterStateMethodNoop, RegisterStateMethodStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type StateMethodArgs<'n, Action, T, Args, Ret> = (&'n str, fn(&T, Args) -> Ret, PhantomData<Action>);

impl<Action, T, Args, Ret> RegisterStateMethodNoop for Wrap<(marker::State, marker::Method), StateMethodArgs<'_, Action, T, Args, Ret>> 
where
    Action: spru::Action<State: spru::State>,
{ 
    type Action = Action;
}

impl<Action, T, Args, Ret> RegisterStateMethodStd for &mut Wrap<(marker::State, marker::Method), StateMethodArgs<'_, Action, T, Args, Ret>> 
where
    Action: spru::Action<State: spru::State>,
    T: spru::item::storage::Storable<Action::State>,
    T: Clone + Sync + Send + 'static,
    Args: RegisterUnpacked + Sync + Send + 'static,
    Ret: spru_script::MethodReturn<Action, T: Clone + Send + Sync + 'static> + 'static
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, method, _) = self.take();
        let reg = rhai::FuncRegistration::new(name);
        Args::register_unpacked(registration.registration.rhai, reg, move |ctx: rhai::NativeCallContext<'_>, this: &mut IdT<T>, args: Args| {
            let mut handle = LedgerHandle::from_rhai(&ctx);
            let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };

            let item = ledger.get(*this)
                .map_err(|e| Box::new(rhai::EvalAltResult::ErrorRuntime(format!("{this:?}: {e}").into(), ctx.call_position())))?;
            let ret = method(&*item, args);
            let (ret, actions) = ret.convert();

            for action in actions {
                ledger.enqueue_action(this.untyped(), action);
            }
            ledger.flush()
                .map_err(|e| format!("Failed to flush actions: {e}"))?;

            Ok(ret)
        });
            
        Ok(())
    }
}


expand_foreach! {$
    { pub trait } [RegisterStateFunctionNoop, RegisterStateFunctionStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type StateFunctionArgs<'n, Action, Args, Ret> = (&'n str, fn(Args) -> Ret, PhantomData<Action>);

impl<Action, Args, Ret> RegisterStateFunctionNoop for Wrap<(marker::State, marker::Function), StateFunctionArgs<'_, Action, Args, Ret>> 
where
    Action: spru::Action,
{ 
    type Action = Action;
}

impl<Action, Args, Ret> RegisterStateFunctionStd for &mut Wrap<(marker::State, marker::Function), StateFunctionArgs<'_, Action, Args, Ret>>
where
    Action: spru::Action,
    Args: FromArguments + 'static,
    Ret: Clone + Send + Sync + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, function, _) = self.take();
        if let Some((_, statics_map)) = registration.statics_map.as_mut() {
            #[allow(deprecated)]
            let fn_ptr = rhai::FnPtr::from_fn(name, move |ctx, mut args| {
                // Self parameter is unused
                args.split_off_first_mut();
                let args = Args::from_arguments(&ctx, args)?;
                Ok(rhai::Dynamic::from(function(args)))
            }).map_err(|e| *e)?;

            statics_map.insert(name.into(), fn_ptr.into());
        }

        Ok(())
    }
}


expand_foreach! {$
    { pub trait } [RegisterStateCreateNoop, RegisterStateCreateStd] { {
        type Action: spru::Action;

        fn register_member<Storage>(&mut self, _registration: &mut Registration2<'_, '_>) -> RhaiResult<()> 
        where
            Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
        {
            Ok(())
        }
    } }
}

type StateCreateArgs<'n, Action, T, Args, Create> = (&'n str, fn(Args) -> Create, PhantomData<(Action, T)>);

impl<Action, T, Args, Create> RegisterStateCreateNoop for Wrap<(marker::State, marker::Create), StateCreateArgs<'_, Action, T, Args, Create>> 
where
    Action: spru::Action
{ 
    type Action = Action;
}

impl<Action, T, Args, Create> RegisterStateCreateStd for &mut Wrap<(marker::State, marker::Create), StateCreateArgs<'_, Action, T, Args, Create>> 
where
    Action: spru::Action, 
    T: spru::item::storage::Storable<Action::State>,
    T: Clone + Send + Sync + 'static,
    Args: FromArguments + 'static,
    Create: Into<Action> + 'static,
{
    type Action = Action;

    fn register_member<Storage>(&mut self, registration: &mut Registration2<'_, '_>) -> RhaiResult<()>
    where
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let (name, create, _) = self.take();
        if let Some((_, statics_map)) = registration.statics_map.as_mut() {
            #[allow(deprecated)]
            let fn_ptr = rhai::FnPtr::from_fn(name, move |ctx, mut args| {
                // Self parameter is unused
                args.split_off_first_mut();
                let args = Args::from_arguments(&ctx, args)?;
                let action: Action = create(args).into();

                let mut handle = crate::LedgerHandle::from_rhai(&ctx);
                let mut ledger = unsafe { handle.get_mut::<Storage, Action>() };
                let id = ledger.enqueue_create(action.into());

                ledger.flush()
                    .map_err(|e| format!("Create {}: {e}", std::any::type_name::<T>()))?;

                let idt = id.force_type::<T>();

                Ok(rhai::Dynamic::from(idt))
            }).map_err(|e| *e)?;
            
            statics_map.insert(name.into(), fn_ptr.into());
        }

        Ok(())
    }
}

// https://rhai.rs/book/patterns/references.html
#[derive(Clone)]
struct LedgerHandle {
    pointer: Arc<RwLock<*mut ()>>,
    rhai_lifetime: i64,
}

// SAFETY: LedgerHandle locks all pointer access, the only concern is LedgerHandle does not outlive the original Ledger reference.
unsafe impl Send for LedgerHandle {}
unsafe impl Sync for LedgerHandle {}

impl LedgerHandle {
    pub fn new<'l, Storage, Action>(ledger: &mut spru::interactor::Ledger<'l, Storage, Action>, rhai_lifetime: i64) -> Self {
        let pointer = ledger as *mut spru::interactor::Ledger<'l, Storage, Action>;
        Self {
            pointer: Arc::new(RwLock::new(pointer.cast())),
            rhai_lifetime,
        }
    }

    pub unsafe fn get_mut<'l, 'i, Storage, Action>(&'i mut self) 
        -> LedgerMut<'l, 'i, Storage, Action>
    {
        // if self.rhai_lifetime != lifetime {
        //     panic!("Dangling LedgerHandle (Expected {}, got {lifetime})", self.rhai_lifetime);
        // }

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

#[doc(hidden)]
pub trait FromArguments: Sized {
    fn from_arguments(ctx: &rhai::NativeCallContext<'_>, args: &mut [&mut rhai::Dynamic]) -> Result<Self, rhai::EvalAltResult>;
}

impl FromArguments for () {
    fn from_arguments(ctx: &rhai::NativeCallContext<'_>, mut args: &mut [&mut rhai::Dynamic]) -> Result<Self, rhai::EvalAltResult> {
        extra_args_error(ctx, &mut args)?;
        Ok(())
    }
}

#[doc(hidden)]
pub trait RegisterUnpacked {
    fn register_unpacked<This, Ret, Func>(rhai: &mut rhai::Engine, reg: rhai::FuncRegistration, f: Func) 
    where
        This: Clone + Send + Sync + 'static,
        Ret: Clone + Send + Sync + 'static,
        Func: Fn(rhai::NativeCallContext<'_>, &mut This, Self) -> Result<Ret, Box<rhai::EvalAltResult>> + Send + Sync + 'static,
    ;
}

impl RegisterUnpacked for () {
    fn register_unpacked<This, Ret, Func>(rhai: &mut rhai::Engine, reg: rhai::FuncRegistration, f: Func) 
    where
        This: Clone + Send + Sync + 'static,
        Ret: Clone + Send + Sync + 'static,
        Func: Fn(rhai::NativeCallContext<'_>, &mut This, Self) -> Result<Ret, Box<rhai::EvalAltResult>> + Send + Sync + 'static,
    {
        reg.register_into_engine(rhai, move |ctx: rhai::NativeCallContext<'_>, this: &mut This| {
            f(ctx, this, ())
        });
    }
}

macro_rules! tuple_from_arguments {
    () => {
        
    };
    ($n:tt $first:ident $($nn:tt $rest:ident)*) => {
        impl<$first, $($rest, )*> FromArguments for ($first, $($rest),*) 
        where
            $first: Clone + Sync + Send + 'static,
            $($rest: Clone + Sync + Send + 'static),*
        {
            #[allow(non_snake_case)]
            fn from_arguments(ctx: &rhai::NativeCallContext<'_>, mut args: &mut [&mut rhai::Dynamic]) -> Result<Self, rhai::EvalAltResult> {
                let $first: $first = pop_type(ctx, &mut args)?;
                $(
                    let $rest: $rest = pop_type(ctx, &mut args)?;
                )*
                Ok(($first, $($rest),*))
            }
        }

        impl<$first, $($rest),*> RegisterUnpacked for ($first, $($rest),*)
        where 
            $first: Clone + Send + Sync + 'static,
            $($rest: Clone + Send + Sync + 'static, )*
        {
            #[allow(non_snake_case)]
            fn register_unpacked<This, Ret, Func>(rhai: &mut rhai::Engine, reg: rhai::FuncRegistration, f: Func) 
                -> ()
            where
                This: Clone + Send + Sync + 'static,
                Ret: Clone + Send + Sync + 'static,
                Func: Fn(rhai::NativeCallContext<'_>, &mut This, Self) -> Result<Ret, Box<rhai::EvalAltResult>> + Send + Sync + 'static,
            {
                reg.register_into_engine(rhai, move |ctx: rhai::NativeCallContext<'_>, this: &mut This, $first: $first, $($rest: $rest),*| {
                    f(ctx, this, (
                        $first, $($rest, )*
                    ))
                });
            }
        }

        tuple_from_arguments!($($nn $rest)*);
    };
}

tuple_from_arguments!(15 P 14 O 13 N 12 M 11 L 10 K 9 J 8 I 7 H 6 G 5 F 4 E 3 D 2 C 1 B 0 A);

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

fn write_scriptable_path(s: &mut String, type_path: &spru_script::ScriptablePath) {
    use std::fmt::Write as _;

    let &spru_script::ScriptablePath(path, type_args) = type_path;

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

#[cfg(test)]
mod test {
    // use super::*;
    use super::{Wrap, Registration1};
    #[allow(unused_imports)]
    use super::{RegisterTypeNoop as _, RegisterTypeStd as _};
    #[allow(unused_imports)]
    use super::{RegisterTypeGetNoop as _, RegisterTypeGetStd as _};

    mod m {
        #[derive(Clone)]
        pub struct S {
            pub i: i32,
        }
    }

    #[test]
    fn t() {
        // let mut rhai = rhai::Engine::new();
        // let mut reg1 = Registration1::new(&mut rhai);
        // (&&&Wrap::<m::S>::new()).register_type(&mut reg1).unwrap();
        // let mut reg2 = reg1.type_registration(Some(spru_script::scriptable_path!(m::S)));
        // (&&&Wrap::<(m::S, i32)>::new()).register_type_get(&mut reg2, "i", |t| t.i).unwrap();
        
        
        // reg2.apply();
    }
}