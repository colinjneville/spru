use std::{any::{Any, TypeId}, collections::HashMap, sync::{Arc, Mutex}};

use crate::scripting::{self, lua};

#[derive(Clone)]
pub struct Lua {
    lua: mlua::Lua,
    mapping_fn_cache: Arc<Mutex<HashMap<TypeId, Box<dyn Any>>>>,
}

impl std::fmt::Debug for Lua {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            lua,
            mapping_fn_cache: _mapping_fn_cache,
        } = self;

        f.debug_struct("Instance")
            .field("lua", lua)
            .field("mapping_fn_cache", &())
            .finish()
    }
}

impl Lua {
    pub fn new() -> Self {
        let lua = mlua::Lua::new();  

        Self {
            lua,
            mapping_fn_cache: Default::default(),
        }
    }

    pub(crate) fn exec<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
        &self, 
        interactor: &spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> mlua::Result<R> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: scripting::ScriptableState<Action, lua::Registry>,
        Action: spru::Action + 'static,
        Context: spru::interactor::GetRoot<Root: lua::IntoLua + 'static>,
    {
        let ledger = interactor.ledger();
        let root = interactor.root();
        self.exec_internal(ledger, Some(root), script)
    }

    pub(crate) fn exec_no_root<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
        &self, 
        interactor: &spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
    ) -> mlua::Result<R> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: scripting::ScriptableState<Action, lua::Registry>,
        Action: spru::Action + 'static,
    {
        let ledger = interactor.ledger();
        self.exec_internal::<Storage, Action, i32, R>(ledger, None, script)
    }

    fn exec_internal<Storage, Action, Root, R: mlua::FromLuaMulti>(
        &self, 
        ledger: &spru::interactor::Ledger<'_, Storage, Action>,
        root: Option<&Root>,
        script: &str,
    ) -> mlua::Result<R> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: scripting::ScriptableState<Action, lua::Registry>,
        Action: spru::Action + 'static,
        Root: lua::IntoLua + 'static,
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

        let registry = lua::Registry::new(self.lua.clone());
        let mut registration = lua::Registration::new();
        <Storage::State as scripting::ScriptableState<Action, lua::Registry>>::register(&registry, &mut registration)?;

        let mapping_fn = registration.into_mapping_fn();

        // Insert a scoped reference to the spru Ledger for lua to access 
        let ledger = lua::Ledger::new(ledger, &mapping_fn);

        let r = self.lua.scope(|scope| {
            let ud = scope.create_userdata(ledger)?;
            self.lua.globals().set(lua::key::LEDGER_GLOBAL, ud)?;

            // Root must be inserted *after* all our field/method registration,
            // so we can't set once and forget in the constructor, so make it part of the scope.
            if let Some(root) = root {
                let root = scope.create_any_userdata_ref(root)?;
                self.lua.globals().set(lua::key::ROOT_GLOBAL, root)?;
            }

            let r = self.lua
                .load(script)
                .eval()?;

            Ok(r)
        })?;

        Ok(r)
    }

    pub fn script<S: ToString>(&self, script: S) -> lua::Script {
        lua::Script::new(self.clone(), script.to_string())
    }
}