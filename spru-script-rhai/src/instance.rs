use std::{any::{Any, TypeId}, sync::{RwLock, RwLockReadGuard, RwLockWriteGuard}};

use append_only_vec::AppendOnlyVec;
use derive_where::derive_where;


/// Cache for rhai instances based on type paramters.
/// In most cases, this will act as a lazy singleton, but servers and test executables 
/// could have co-mingled game types.
#[derive(Debug)]
struct RhaiCache {
    map: AppendOnlyVec<(TypeId, crate::Settings, Internal)>,
}

impl RhaiCache {
    const fn new() -> Self {
        Self {
            map: AppendOnlyVec::new(),
        }
    }

    fn get<Storage, Action, Lexicon>(&'static self, settings: &crate::Settings) -> Cached 
    where 
        Storage: spru::item::Storage,
        Action: spru::Action<State = Storage::State> + 'static,
        Lexicon: spru_script_base::Lexicon<Language = Rhai<Action, Lexicon>>,
    {
        let type_id = Self::key::<Storage, Action>();

        for (cached_type_id, cached_settings, internal) in self.map.iter() {

            if *cached_type_id == type_id && cached_settings == settings {
                return Cached { internal };
            }
        }

        let index = self.map.push((type_id, settings.clone(), Internal::init::<Storage, Action, Lexicon>(settings)));
        Cached { internal: &self.map[index].2 }
    }

    fn key<Storage, Action>() -> TypeId {
        Any::type_id(&|| {})
    }
}

static RHAI_CACHE: RhaiCache = RhaiCache::new();

#[derive(Debug)]
pub(crate) struct Internal {
    rhai: RwLock<rhai::Engine>,
    scope: rhai::Scope<'static>,
}


impl Internal {
    fn init<Storage, Action, Lexicon>(_settings: &crate::Settings) -> Self 
    where
        Storage: spru::item::Storage,
        Action: spru::Action<State = Storage::State> + 'static,
        Lexicon: spru_script_base::Lexicon<Language = Rhai<Action, Lexicon>>,
    {
        let mut rhai = rhai::Engine::new();
        rhai.set_max_expr_depths(64, 32);

        // Needed for custom equality handling?
        rhai.set_fast_operators(false);
        
        rhai.register_type::<spru::player::Id>();
        rhai.register_fn("==", |pid: &mut spru::player::Id, pid2: spru::player::Id| {
            *pid == pid2
        });
        rhai.register_fn("!=", |pid: &mut spru::player::Id, pid2: spru::player::Id| {
            *pid != pid2
        });

        fn from_int<T: TryFrom<rhai::INT>>(
            ctx: rhai::NativeCallContext<'_>, 
            value: &mut rhai::INT,
        ) -> Result<T, Box<rhai::EvalAltResult>> {
            T::try_from(*value)
                .map_err(|_| rhai::EvalAltResult::ErrorDataTooLarge("Value is too large for the target type".to_string(), ctx.call_position()))
                .map_err(Box::new)
        }

        fn to_int<T: TryInto<rhai::INT> + Copy>(
            ctx: rhai::NativeCallContext<'_>, 
            value: &mut T,
        )-> Result<rhai::INT, Box<rhai::EvalAltResult>> {
            (*value).try_into()
                .map_err(|_| rhai::EvalAltResult::ErrorDataTooLarge("Value is too large for the target type".to_string(), ctx.call_position()))
                .map_err(Box::new)
        }

        rhai.register_fn("to_u8", from_int::<u8>);
        rhai.register_fn("to_u16", from_int::<u16>);
        rhai.register_fn("to_u32", from_int::<u32>);
        rhai.register_fn("to_u64", from_int::<u64>);
        rhai.register_fn("to_usize", from_int::<usize>);
        rhai.register_fn("to_i8", from_int::<i8>);
        rhai.register_fn("to_i16", from_int::<i16>);
        rhai.register_fn("to_i32", from_int::<i32>);
        rhai.register_fn("to_i64", from_int::<i64>);
        rhai.register_fn("to_isize", from_int::<isize>);

        rhai.register_fn("to_int", to_int::<u8>);
        rhai.register_fn("to_int", to_int::<u16>);
        rhai.register_fn("to_int", to_int::<u32>);
        rhai.register_fn("to_int", to_int::<u64>);
        rhai.register_fn("to_int", to_int::<usize>);
        rhai.register_fn("to_int", to_int::<i8>);
        rhai.register_fn("to_int", to_int::<i16>);
        if std::any::TypeId::of::<rhai::INT>() != std::any::TypeId::of::<i32>() {
            rhai.register_fn("to_int", to_int::<i32>);
        }
        if std::any::TypeId::of::<rhai::INT>() != std::any::TypeId::of::<i64>() {
            rhai.register_fn("to_int", to_int::<i64>);
        }
        rhai.register_fn("to_int", to_int::<isize>);
        
        let mut registration = crate::Registration1::new(&mut rhai);
        Lexicon::register::<Storage>(&mut registration);
        let scope = registration.apply();

        Self {
            rhai: RwLock::new(rhai),
            scope,
        }
    }
}


#[derive(Debug, Clone)]
pub(crate) struct Cached {
    internal: &'static Internal,
}

impl Cached {
    pub(crate) fn rhai_mut(&self) -> RwLockWriteGuard<'_, rhai::Engine> {
        self.internal.rhai.write()
            .expect("Rhai lock is poisoned")
    }
}


#[derive(serde::Serialize, serde::Deserialize)]
#[derive_where(Debug, Clone; )]
#[serde(default)]
#[serde(bound(serialize = "", deserialize = "Action: 'static, Lexicon: 'static"))]
pub struct Rhai<Action, Lexicon> {
    settings: crate::Settings,
    #[serde(skip)]
    _p: std::marker::PhantomData<(Action, Lexicon)>,
}

impl<Action: 'static, Lexicon: 'static> Default for Rhai<Action, Lexicon> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<Action: 'static, Lexicon: 'static> Rhai<Action, Lexicon> {
    pub fn new(settings: crate::Settings) -> Self {
        Self {
            settings,
            _p: std::marker::PhantomData,
        }
    }
}


impl<Action, Lexicon> spru_script_base::Language for Rhai<Action, Lexicon>
where
    Action: spru::Action,
{
    type Action = Action;

    type Registration<'r> = crate::Registration1<'r>;
    type Error = Box<rhai::EvalAltResult>;
}

impl<Action, Lexicon, Args, Ret, Context, Output> spru_script_base::LanguageExec<Args, Ret, Context, Output> for Rhai<Action, Lexicon> 
where
    Action: spru::Action,
    Lexicon: spru_script_base::Lexicon<Language = Self>,
    Context: crate::Context,
    Output: crate::Output<Ret>,
    Output::RetIn: Clone + Send + Sync + 'static,
    Args: Clone + Send + Sync + 'static,
{
    fn exec<Storage>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
        args: Args,
    ) 
        -> Result<Ret, Self::Error> 
    where
        Storage: spru::item::Storage<State = Action::State>,
    {
        let spru::interactor::SplitMut {
            ledger,
            context,
            output,
        } = interactor.split_mut();

        let ledger_handle = crate::LedgerHandle::new(ledger, 0);

        let cached = RHAI_CACHE.get::<Storage, Action, Lexicon>(&self.settings);
        let mut rhai = cached.rhai_mut();

        let mut context_map = rhai::Map::new();
        context.apply(&mut context_map);
        
        let mut output_map = rhai::Map::new();
        output.apply(&mut output_map);

        let mut scope = cached.internal.scope.clone();
        scope.push(&*crate::key::GLOBAL_CONTEXT, context_map);
        scope.push(&*crate::key::GLOBAL_OUTPUT, output_map);
        scope.push(&*crate::key::GLOBAL_ARGS, args);

        // Store a pointer to the Ledger for the duration of the script
        rhai.set_default_tag(rhai::Dynamic::from(ledger_handle));
        
        let ret = rhai
            .eval_with_scope::<Output::RetIn>(&mut scope, script)?;

        // Pointer to the Ledger must be cleared to prevent it from outliving the mut borrow 
        rhai.set_default_tag(());

        let mut output_map = scope.remove::<rhai::Map>(&crate::key::GLOBAL_OUTPUT)
            .expect("Expected Output map");

        output.triggers(&mut output_map);

        Ok(output.apply_ret(ret))
    }
}
