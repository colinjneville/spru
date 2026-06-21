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

    fn get<Storage, Lexicon>(&'static self, settings: &crate::Settings) -> Cached 
    where 
        Storage: spru::item::Storage,
        Lexicon: spru_script::Lexicon<Language = Rhai, Action: spru::Action<State = Storage::State>>,
    {
        let type_id = Self::key::<Storage, Lexicon>();

        if let Some(cached) = self.get_internal(type_id, settings) {
            cached
        } else {
            let index = self.map.push((type_id, settings.clone(), Internal::init::<Storage, Lexicon>(settings)));
            Cached { internal: &self.map[index].2 }
        }
    }

    fn get_stateless<Lexicon>(&'static self, settings: &crate::Settings) -> Cached
    where
        Lexicon: spru_script::StatelessLexicon<Language = Rhai>,
    {
        // TODO we can potentially reuse an existing <Storage, Lexicon> instance as a 
        // stateless <Lexicon> instance without issue
        let type_id = Self::key::<(), Lexicon>();

        if let Some(cached) = self.get_internal(type_id, settings) {
            cached
        } else {
            let index = self.map.push((type_id, settings.clone(), Internal::init_stateless::<Lexicon>(settings)));
            Cached { internal: &self.map[index].2 }
        }
    }

    fn get_internal(&'static self, type_id: TypeId, settings: &crate::Settings) -> Option<Cached> {
        for (cached_type_id, cached_settings, internal) in self.map.iter() {
            if *cached_type_id == type_id && cached_settings == settings {
                return Some(Cached { internal });
            }
        }

        None
    }

    fn key<Storage, Lexicon>() -> TypeId {
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
    fn init_stateless<Lexicon>(settings: &crate::Settings) -> Self 
    where
        Lexicon: spru_script::StatelessLexicon<Language = Rhai>,
    {
        let mut rhai = Self::init_internal(settings);
        let mut registration = crate::Registration1::new(&mut rhai);
        Lexicon::register_stateless(&mut registration);
        let scope = registration.apply();

        Self {
            rhai: RwLock::new(rhai),
            scope,
        }
    }

    fn init_internal(_settings: &crate::Settings) -> rhai::Engine {
        let mut rhai = rhai::Engine::new();
        rhai.set_max_expr_depths(64, 32);

        // Needed for custom equality handling?
        rhai.set_fast_operators(false);


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

        rhai
    }

    fn init<Storage, Lexicon>(settings: &crate::Settings) -> Self 
    where
        Storage: spru::item::Storage,
        Lexicon: spru_script::Lexicon<Language = Rhai, Action: spru::Action<State = Storage::State>>,
    {
        let mut rhai = Self::init_internal(settings);
        
        rhai.register_type::<spru::player::Id>();
        rhai.register_fn("==", |pid: &mut spru::player::Id, pid2: spru::player::Id| {
            *pid == pid2
        });
        rhai.register_fn("!=", |pid: &mut spru::player::Id, pid2: spru::player::Id| {
            *pid != pid2
        });

        let mut registration = crate::Registration1::new(&mut rhai);
        Lexicon::register_stateless(&mut registration);
        Lexicon::register_state::<Storage>(&mut registration);
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
    pub(crate) fn rhai_ref(&self) -> RwLockReadGuard<'_, rhai::Engine> {
        self.internal.rhai.read()
            .expect("Rhai lock is poisoned")
    }

    pub(crate) fn rhai_mut(&self) -> RwLockWriteGuard<'_, rhai::Engine> {
        // TODO this only needs to be mutable to set the default tag, which could
        // probably removed by adding it to the scope instead
        self.internal.rhai.write()
            .expect("Rhai lock is poisoned")
    }
}

pub struct Rhai;

impl spru_script::LanguageActual for Rhai {
    type Registration<'r> = crate::Registration1<'r>;
}

#[derive(serde::Serialize, serde::Deserialize)]
#[derive_where(Debug, Clone; )]
#[serde(default)]
#[serde(bound(serialize = "", deserialize = "Lexicon: 'static"))]
pub struct RhaiInstance<Lexicon> {
    settings: crate::Settings,
    #[serde(skip)]
    _p: std::marker::PhantomData<(Lexicon, )>,
}

impl<Lexicon: 'static> Default for RhaiInstance<Lexicon> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<Lexicon: 'static> RhaiInstance<Lexicon> {
    pub fn new(settings: crate::Settings) -> Self {
        Self {
            settings,
            _p: std::marker::PhantomData,
        }
    }
}

impl<Lexicon> spru_script::StatelessLanguage for RhaiInstance<Lexicon> {
    type Error = Box<rhai::EvalAltResult>;
}

impl<Lexicon> spru_script::Language for RhaiInstance<Lexicon>
where
    Lexicon: spru_script::Lexicon<Language = Rhai>,
{
    type Action = Lexicon::Action;
}

impl<Lexicon, Args, Ret> spru_script::LanguageStatelessEval<Args, Ret> for RhaiInstance<Lexicon> 
where
    Lexicon: spru_script::StatelessLexicon<Language = Rhai>,
    Ret: Clone + Send + Sync + 'static,
    Args: Clone + Send + Sync + 'static,
{
    fn stateless_eval(&self, script: &str, args: Args) -> Result<Ret, Self::Error> {
        let cached = RHAI_CACHE.get_stateless::<Lexicon>(&self.settings);
        let rhai = cached.rhai_ref();

        let mut scope = cached.internal.scope.clone();
        scope.push(&*crate::key::GLOBAL_ARGS, args);
        
        let ret = rhai
            .eval_with_scope::<Ret>(&mut scope, script)?;

        Ok(ret)
    }
}

impl<Lexicon, Args, Ret, Root> spru_script::LanguageEval<Args, Ret, Root> for RhaiInstance<Lexicon>
where
    Lexicon: spru_script::Lexicon<Language = Rhai>,
    Ret: Clone + Send + Sync + 'static,
    Args: Clone + Send + Sync + 'static,
    Root: Clone + Send + Sync + 'static,
{
    fn eval<Storage>(&self, storage: &Storage, root: &Root, script: &str, args: Args) 
        -> Result<Ret, Self::Error>
    where 
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let ledger_handle = crate::LedgerHandle::new_readonly::<Storage>(storage);

        let cached = RHAI_CACHE.get::<Storage, Lexicon>(&self.settings);
        let mut rhai = cached.rhai_mut();

        let mut scope = cached.internal.scope.clone();
        scope.push(&*crate::key::GLOBAL_ARGS, args);
        scope.push(&*crate::key::GLOBAL_CONTEXT, rhai::Map::from([("root".into(), rhai::Dynamic::from(root.clone()))]));

         // Store a pointer to the Ledger for the duration of the script
        rhai.set_default_tag(rhai::Dynamic::from(ledger_handle));
        
        let ret = rhai
            .eval_with_scope::<Ret>(&mut scope, script)?;

        // Pointer to the Ledger must be cleared to prevent it from outliving the mut borrow 
        rhai.set_default_tag(());

        Ok(ret)        
    }
}

impl<Lexicon, Args, Ret, Context, Output> spru_script::LanguageExec<Args, Ret, Context, Output> for RhaiInstance<Lexicon> 
where
    Lexicon: spru_script::Lexicon<Language = Rhai>,
    Context: crate::Context,
    Output: crate::Output<Ret>,
    Output::RetIn: Clone + Send + Sync + 'static,
    Args: Clone + Send + Sync + 'static,
{
    fn exec<Storage>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Lexicon::Action, Context, Output>,
        script: &str,
        args: Args,
    ) 
        -> Result<Ret, Self::Error> 
    where
        Storage: spru::item::Storage<State = <Lexicon::Action as spru::Action>::State>,
    {
        let spru::interactor::SplitMut {
            ledger,
            context,
            output,
        } = interactor.split_mut();

        let ledger_handle = crate::LedgerHandle::new(ledger);

        let cached = RHAI_CACHE.get::<Storage, Lexicon>(&self.settings);
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
