mod context;
pub use context::Context;
pub(crate) mod func;
mod instance;
pub use instance::Rhai;
pub(crate) mod key;
mod output;
pub use output::Output;
mod settings;
pub use settings::Settings;
mod registration;
pub use registration::{Registration, RegistrationState, RegistrationType};
mod registry;
pub use registry::Registry;

use std::{cell::RefCell, marker::PhantomData, sync::{Arc, RwLock, RwLockWriteGuard, atomic::{self, AtomicI64}}};

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
        -> ()
    where
        This: Clone + Send + Sync + 'static,
        Ret: Clone + Send + Sync + 'static,
        Func: Fn(rhai::NativeCallContext<'_>, &mut This, Self) -> Result<Ret, Box<rhai::EvalAltResult>> + Send + Sync + 'static,
    ;
}

impl RegisterUnpacked for () {
    fn register_unpacked<This, Ret, Func>(rhai: &mut rhai::Engine, reg: rhai::FuncRegistration, f: Func) 
        -> ()
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

fn extra_args_error(ctx: &rhai::NativeCallContext<'_>, args: &[&mut rhai::Dynamic]) -> Result<(), rhai::EvalAltResult> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(rhai::EvalAltResult::ErrorRuntime("Too many arguments".into(), ctx.call_position()))
    }
}