use std::any::TypeId;

use append_only_vec::AppendOnlyVec;

/// Cache for lua instances based on type paramters.
/// In most cases, this will act as a lazy singleton, but servers and test executables 
/// could have co-mingled game types.
#[derive(Debug)]
struct LuaCache {
    map: AppendOnlyVec<(TypeId, Internal)>,
}

impl LuaCache {
    const fn new() -> Self {
        Self {
            map: AppendOnlyVec::new(),
        }
    }

    fn get<State: 'static, Action: 'static>(&'static self) -> Cached {
        let type_id = TypeId::of::<(State, Action)>();

        for (tid, internal) in self.map.iter() {
            if *tid == type_id {
                return Cached { internal };
            }
        }

        let index = self.map.push((type_id, Internal::init::<State, Action>()));
        Cached { internal: &self.map[index].1 }
    }
}

static LUA_CACHE: LuaCache = LuaCache::new();

#[derive(Debug)]
pub(crate) struct Internal {
    lua: mlua::Lua,
    type_parameters_metatable: mlua::Table,
}

impl Internal {
    fn init<State: 'static, Action: 'static>() -> Self {
        // In the future, we could possibly do some preemptive setup based on the 
        // type parameters here, or have a global settings to customize new initializations.

        let lua = mlua::Lua::new();
        let type_parameters_metatable = lua.create_table()
            .expect("Failed to create type parameters metatable");

        let type_parameters_index_metafunction = lua.create_function(|_, (table, key): (mlua::Table, mlua::Value)| {
            for pair in table.pairs::<mlua::Value, mlua::Value>() {
                let (k, v) = pair?;

                if let Some(k_table) = k.as_table() {
                    // Special-case single parameter types to allow just the type parameter table, 
                    // instead of a table of the type parameter table (e.g. `X[i32]` instead of `X[[i32]]`).
                    if k_table.raw_len() == 1 {
                        if k_table.raw_get::<mlua::Value>(1)? == key {
                            return Ok(v);
                        }
                    }

                    if let Some(key_table) = key.as_table() {
                        // Compare structural equality of the type parameter tables
                        if k_table.raw_len() == key_table.raw_len() {
                            let mut all_match = true;
                            for i in 1..=k_table.raw_len() {
                                if k_table.raw_get::<mlua::Value>(i)? != key_table.raw_get::<mlua::Value>(i)? {
                                    all_match = false;
                                    break;
                                }
                            }

                            if all_match {
                                return Ok(v);
                            }
                        }
                    }
                }
            }

            Ok(mlua::Value::Nil)
        }).expect("Failed to create type parameters equality function");

        type_parameters_metatable.set(mlua::MetaMethod::Index.name(), type_parameters_index_metafunction)
            .expect("Failed to create type parameters metatable");

        Self {
            lua,
            type_parameters_metatable,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cached {
    internal: &'static Internal,
}

impl Cached {
    pub(crate) fn lua(&self) -> &mlua::Lua {
        &self.internal.lua
    }

    pub(crate) fn type_parameters_metatable(&self) -> &mlua::Table {
        &self.internal.type_parameters_metatable
    }
}

#[derive(Debug, Clone)]
pub struct Lua<State, Action> {
    cached: Cached,
    _p: std::marker::PhantomData<(State, Action)>,
}

impl<State: 'static, Action: 'static> Default for Lua<State, Action> {
    fn default() -> Self {
        Self::new()
    }
}

impl<State: 'static, Action: 'static> Lua<State, Action> {
    pub fn new() -> Self {
        Self {
            cached: LUA_CACHE.get::<State, Action>(),
            _p: std::marker::PhantomData,
        }
    }

    fn exec_internal<Storage, Root, Return>(
        &self, 
        ledger: &mut spru::interactor::Ledger<'_, Storage, Action>,
        root: Option<&Root>,
        script: &str,
    ) -> mlua::Result<Return> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: spru_script::ScriptableState<Action, crate::Registry>,
        Action: spru::Action,
        Root: crate::IntoLua + 'static,
        // Required because mlua's setters require &mut (understandably), even though we don't mutate the IdTs themselves
        Root: Clone,
        Return: mlua::FromLuaMulti,
    {
        // TODO Need to find a way to get caching to work with non-static Storage
        // right now we are re-registering the whole tree each exec...

        // let mut cache = self.mapping_fn_cache.lock()
        //     .expect("Lua mapping mutex poisoned");

        // let closure_type = |_: &Storage, _: &Action| { };

        // fn type_id_of_val<T: Any>(_t: &T) -> TypeId {
        //     TypeId::of::<T>()
        // }

        // let closure_type_key = type_id_of_val(&closure_type);

        // let mapping_fn = match cache.entry(closure_type_key) {
        //     std::collections::hash_map::Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
        //     std::collections::hash_map::Entry::Vacant(vacant_entry) => {
        //         let mut registry = LuaRegistry::new(self.lua.clone());
        //         <Storage::State as ScriptableState<LuaRegistry<Storage, Action>>>::register(&mut registry)?;

        //         vacant_entry.insert(Box::new(registry.into_mapping_fn()))
        //     },
        // };

        // let mapping_fn: &MappingFn<Storage, Action> = mapping_fn.downcast_ref()
        //     .expect("MappingFn type mismatch");

        let registry = crate::Registry::new(self.cached.clone());
        let mut registration = crate::Registration::new();
        <Storage::State as spru_script::ScriptableState<Action, crate::Registry>>::register(&registry, &mut registration)?;

        let (get_mapping_fn, set_mapping_fn, method_mapping_fn, create_mapping_fn) = registration.into_mapping_fns();

        // Insert a scoped reference to the spru Ledger for lua to access 
        let ledger = crate::Ledger::new(ledger, &get_mapping_fn, &set_mapping_fn, &method_mapping_fn, &create_mapping_fn);

        let mut root_clone = root.cloned();

        let r = self.cached.lua().scope(|scope| {
            let ud = scope.create_userdata(ledger)?;
            self.cached.lua().globals().set(crate::key::LEDGER_GLOBAL, ud)?;

            // Root must be inserted *after* all our field/method registration,
            // so we can't set once and forget in the constructor, so make it part of the scope.
            if let Some(root_clone) = &mut root_clone {
                let root = scope.create_any_userdata_ref_mut(root_clone)?;
                self.cached.lua().globals().set(crate::key::ROOT_GLOBAL, root)?;
            }

            let r = self.cached.lua()
                .load(script)
                .eval()?;

            Ok(r)
        })?;

        Ok(r)
    }
}

impl<State, Action, Return> spru_script::LanguageNoRoot<State, Action, Return> for Lua<State, Action> 
where
    State: 'static,
    Return: mlua::FromLuaMulti,
{
    type Registry = crate::Registry;
    type Error = mlua::Error;

    /// Execute a script without access to the Game's Root. 
    /// Mainly useful for the Game Init, where no root exists.
    fn exec_no_root<Storage, Context, Output>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> Result<Return, Self::Error>
    where
        Storage: spru::item::Storage<State = State>,
        State: spru::State + spru_script::ScriptableState<Action, Self::Registry>,
        Action: spru::Action<State = State>,
    {
        let ledger = interactor.ledger_mut();
        self.exec_internal::<Storage, i32, Return>(ledger, None, script)
    }
}

impl<State, Action, Return, Root> spru_script::Language<State, Action, Return, Root> for Lua<State, Action> 
where
    State: 'static,
    // Clone required because mlua's setters require &mut (understandably), even though we don't mutate the IdTs themselves
    Root: crate::IntoLua + Clone + 'static,
    Return: mlua::FromLuaMulti,
{
    /// Execute a script with access to the Game's Root. 
    fn exec<Storage, Context, Output>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> Result<Return, Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
        State: spru::State + spru_script::ScriptableState<Action, Self::Registry>,
        Action: spru::Action<State = State>,
        Context: spru::interactor::GetRoot<Root = Root>,
    {
        let spru::interactor::SplitMut {
            ledger,
            context,
            output: _output,
        } = interactor.split_mut();
        
        self.exec_internal(ledger, Some(context.get_root()), script)
    }
}