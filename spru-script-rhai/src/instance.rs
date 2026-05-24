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

    fn get<Storage, Action>(&'static self, settings: &crate::Settings) -> Cached 
    where 
        Storage: spru::item::Storage,
        Storage::State: spru_script::Scriptable<Action, crate::Registry>,
        Action: spru::Action + 'static,
    {
        let type_id = Self::key::<Storage, Action>();

        for (cached_type_id, cached_settings, internal) in self.map.iter() {

            if *cached_type_id == type_id && cached_settings == settings {
                return Cached { internal };
            }
        }

        let index = self.map.push((type_id, settings.clone(), Internal::init::<Storage, Action>(settings)));
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
    fn init<Storage, Action>(_settings: &crate::Settings) -> Self 
    where
        Storage: spru::item::Storage,
        Storage::State: spru_script::Scriptable<Action, crate::Registry>,
        Action: spru::Action + 'static,
    {
        // In the future, we could possibly do some preemptive setup based on the 
        // type parameters here, or have a global settings to customize new initializations.

        let mut rhai = rhai::Engine::new();

        // Needed for custom equality handling?
        rhai.set_fast_operators(false);
        
        rhai.register_fn(rhai::OP_EQUALS, |pid: &mut spru::player::Id, pid2: spru::player::Id| {
            *pid == pid2
        });

        let registry = crate::Registry::new();
        let mut registration = crate::Registration::new(&mut rhai);

        <crate::Registry as spru_script::RegistryType<Storage::State, Action, spru::player::Id>>::register_type::<Storage>(
            &registry, 
            &mut registration, 
            Some(spru_script::scriptable_path!(spru::player::Id))
        )
            .expect("spru::player::Id registration failed");
        
        <Storage::State as spru_script::Scriptable<Action, crate::Registry>>::register::<Storage>(&registry, &mut registration)
            .expect("Scripting registration failed");

        let mut scope = rhai::Scope::new();
        for (base_segment, map) in registration.globals {
            scope.push(base_segment, map);
        }
        scope.push(&*crate::key::GLOBAL_TYPE, registration.static_maps);

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
    pub(crate) fn rhai(&self) -> RwLockReadGuard<'_, rhai::Engine> {
        self.internal.rhai.read()
            .expect("Rhai lock is poisoned")
    }

    pub(crate) fn rhai_mut(&self) -> RwLockWriteGuard<'_, rhai::Engine> {
        self.internal.rhai.write()
            .expect("Rhai lock is poisoned")
    }
}


#[derive(serde::Serialize, serde::Deserialize)]
#[derive_where(Debug, Clone; )]
#[serde(default)]
#[serde(bound(serialize = "", deserialize = "State: 'static, Action: 'static"))]
pub struct Rhai<State, Action> {
    settings: crate::Settings,
    #[serde(skip)]
    _p: std::marker::PhantomData<(State, Action)>,
}

impl<State: 'static, Action: 'static> Default for Rhai<State, Action> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<State: 'static, Action: 'static> Rhai<State, Action> {
    pub fn new(settings: crate::Settings) -> Self {
        Self {
            settings,
            _p: std::marker::PhantomData,
        }
    }
}


impl<State, Action> spru_script::LanguageBase<State, Action> for Rhai<State, Action> {
    type Registry = crate::Registry;
    type Error = Box<rhai::EvalAltResult>;
}

impl<State, Action, Args, Ret, Context, Output> spru_script::Language<State, Action, Args, Ret, Context, Output> for Rhai<State, Action> 
where
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
        Storage: spru::item::Storage<State = State>,
        State: spru::State + spru_script::Scriptable<Action, Self::Registry>,
        Action: spru::Action<State = State>,
    {
        let spru::interactor::SplitMut {
            ledger,
            context,
            output,
        } = interactor.split_mut();

        let ledger_handle = crate::LedgerHandle::new(ledger, 0);

        let cached = RHAI_CACHE.get::<Storage, Action>(&self.settings);
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

        let mut output_map = scope.get_mut(&crate::key::GLOBAL_OUTPUT)
            .expect("Expected Output map")
            .as_map_mut()
            .expect("Expected Output map");

        output.triggers(&mut *output_map);

        Ok(output.apply_ret(ret))
    }
}
