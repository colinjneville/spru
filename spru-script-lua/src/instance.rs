use std::any::TypeId;

use append_only_vec::AppendOnlyVec;
use derive_where::derive_where;

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
    // TODO this should just be moved to Registry, since the current model is type registration is
    // only ever a one-time event.
    type_parameters_metatable: mlua::Table,
}

impl Internal {
    fn init<State: 'static, Action: 'static>() -> Self {
        // In the future, we could possibly do some preemptive setup based on the 
        // type parameters here, or have a global settings to customize new initializations.

        let lua = mlua::Lua::new();
        let type_parameters_metatable = lua.create_table()
            .expect("Failed to create type parameters metatable");

        lua.register_userdata_type::<spru::player::Id>(|registry| {
            use mlua::UserDataMethods as _;
            registry.add_meta_method(mlua::MetaMethod::Eq.name(), |lua, a, b: mlua::MultiValue| {
                let b: spru::player::Id = crate::FromLuaMulti::from_lua_multi(b, lua)?;
                Ok(a == &b)
            });
        }).expect("spru::player::Id registration failed");

        let type_parameters_index_metafunction = lua.create_function(Self::type_parameters_index_metafunction)
            .expect("Failed to create type parameters index function");

        type_parameters_metatable.set(mlua::MetaMethod::Index.name(), type_parameters_index_metafunction)
            .expect("Failed to create type parameters metatable");

        let type_parameters_tostring_metafunction = lua.create_function(Self::type_parameters_tostring_metafunction)
            .expect("Failed to create type parameters index function");

        type_parameters_metatable.set(mlua::MetaMethod::ToString.name(), type_parameters_tostring_metafunction)
            .expect("Failed to create type parameters metatable");

        Self {
            lua,
            type_parameters_metatable,
        }
    }

    fn type_parameters_tostring_metafunction(_lua: &mlua::Lua, table: mlua::Table)
        -> mlua::Result<String>
    {
        table.get("__type_path")
    }

    fn type_parameters_index_metafunction(_lua: &mlua::Lua, (table, key): (mlua::Table, mlua::Value))
        -> mlua::Result<mlua::Value>
    {
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

        use mlua::ObjectLike as _;

        let mut message = format!("Unknown type parameter '{}' for '{}'\nAvailable type parameters:", key.to_string()?, table.to_string()?);

        for pair in table.pairs::<mlua::Value, mlua::Value>() {
            let (k, _v) = pair?;

            let is_pathy = match &k {
                mlua::Value::Table(_) => true,
                mlua::Value::String(k_str) if k_str.as_bytes()[0] != b'_' => true,
                _ => false,
            };

            if is_pathy {
                use std::fmt::Write as _;

                write!(message, "\n{}", k.to_string()?).unwrap();
            }
        }

        Err(mlua::Error::runtime(message))
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

#[derive(serde::Serialize, serde::Deserialize)]
#[derive_where(Debug, Clone; )]
#[serde(default)]
#[serde(bound(serialize = "", deserialize = "State: 'static, Action: 'static"))]
pub struct Lua<State, Action> {
    #[serde(skip)]
    cached: Cached,
    #[serde(skip)]
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
}

impl<State, Action> spru_script::LanguageBase<State, Action> for Lua<State, Action> 
where
    State: 'static,
{
    type Registry = crate::Registry;
    type Error = mlua::Error;
}

pub trait Context: Sized {
    type Root;

    fn fill_table<'scope, 'env>(
        &'env mut self,
        lua: &mlua::Lua, 
        scope: &'scope mlua::Scope<'scope, 'env>,
        table: &mut mlua::Table,
    ) 
        -> mlua::Result<()>;
}

impl<Root> Context for spru::interaction::Context<'_, Root> 
where 
    Root: Clone + mlua::MaybeSend + 'static,
{
    type Root = Root;

    fn fill_table<'scope, 'env>(
        &'env mut self,
        lua: &mlua::Lua, 
        _scope: &'scope mlua::Scope<'scope, 'env>,
        table: &mut mlua::Table,
    ) 
        -> mlua::Result<()>
    {
        // Root must be inserted *after* all our field/method registration,
        // so we can't set once and forget in the constructor, so make it part of the scope.
        let root = lua.create_any_userdata(self.root.clone())?;
        table.set(crate::key::CONTEXT_ROOT, root)?;

        let player = crate::IntoLua::into_lua(self.player, lua)?;
        table.set(crate::key::CONTEXT_PLAYER, player)?;

        Ok(())
    }
}

impl<Root> Context for spru::reaction::Context<'_, Root>
where
    Root: Clone + mlua::MaybeSend + 'static,
{
    type Root = Root;
    
    fn fill_table<'scope, 'env>(
        &'env mut self,
        lua: &mlua::Lua, 
        _scope: &'scope mlua::Scope<'scope, 'env>,
        table: &mut mlua::Table,
    ) 
        -> mlua::Result<()> 
    {
        // Root must be inserted *after* all our field/method registration,
        // so we can't set once and forget in the constructor, so make it part of the scope.
        let root = lua.create_any_userdata(self.root.clone())?;
        table.set(crate::key::CONTEXT_ROOT, root)?;

        let player = crate::IntoLua::into_lua(self.player, lua)?;
        table.set(crate::key::CONTEXT_PLAYER, player)?;

        Ok(())
    }
}

impl<Root> Context for spru::player::init::Context<'_, Root>
where
    Root: Clone + mlua::MaybeSend + 'static,
{
    type Root = Root;
    
    fn fill_table<'scope, 'env>(
        &'env mut self,
        lua: &mlua::Lua, 
        _scope: &'scope mlua::Scope<'scope, 'env>,
        table: &mut mlua::Table,
    ) 
        -> mlua::Result<()> 
    {
        // Root must be inserted *after* all our field/method registration,
        // so we can't set once and forget in the constructor, so make it part of the scope.
        let root = lua.create_any_userdata(self.root.clone())?;
        table.set(crate::key::CONTEXT_ROOT, root)?;

        let player = crate::IntoLua::into_lua(self.player, lua)?;
        table.set(crate::key::CONTEXT_PLAYER, player)?;

        Ok(())
    }
}

impl Context for spru::game::init::Context {
    type Root = ();
    
    fn fill_table<'scope, 'env>(
        &'env mut self,
        _lua: &mlua::Lua, 
        _scope: &'scope mlua::Scope<'scope, 'env>,
        _table: &mut mlua::Table,
    ) 
        -> mlua::Result<()> 
    {
        Ok(())
    }
}

pub trait Output<Ret> {
    type RetIn;

    fn create<'scope, 'env>(
        &'env mut self,
        lua: &mlua::Lua, 
        _scope: &'scope mlua::Scope<'scope, 'env>,
    ) 
        -> mlua::Result<mlua::AnyUserData>;

    fn apply_ret(&mut self, _ret: Self::RetIn) -> Ret;
}

impl<Ret, Trigger> Output<Ret> for spru::interaction::Output<Trigger> 
where 
    Trigger: crate::FromLuaMulti,
{
    type RetIn = Ret;

    fn create<'scope, 'env>(
        &'env mut self,
        _lua: &mlua::Lua, 
        scope: &'scope mlua::Scope<'scope, 'env>,
    ) 
        -> mlua::Result<mlua::AnyUserData>
    {
        let aud = scope.create_any_userdata(self, |register| {
            use mlua::UserDataMethods as _;
            register.add_method_mut(crate::key::OUTPUT_ENQUEUE_TRIGGER, |lua, output, trigger: mlua::MultiValue| {
                let trigger = Trigger::from_lua_multi(trigger, lua)?;
                <Self as spru::interactor::EnqueueTrigger>::enqueue_trigger(*output, trigger);
                Ok(())
            });
        })?;

        Ok(aud)
    }


    fn apply_ret(&mut self, ret: Self::RetIn) -> Ret {
        ret
    }
}

impl<Trigger, GameOutcome> Output<()> for spru::reaction::Output<Trigger, GameOutcome>
where
    Trigger: crate::FromLuaMulti,
{
    type RetIn = Option<GameOutcome>;

    fn create<'scope, 'env>(
        &'env mut self,
        _lua: &mlua::Lua, 
        scope: &'scope mlua::Scope<'scope, 'env>,
    ) 
        -> mlua::Result<mlua::AnyUserData> 
    {
        let aud = scope.create_any_userdata(self, |register| {
            use mlua::UserDataMethods as _;
            register.add_method_mut(crate::key::OUTPUT_ENQUEUE_TRIGGER, |lua, output, trigger: mlua::MultiValue| {
                let trigger = Trigger::from_lua_multi(trigger, lua)?;
                <Self as spru::interactor::EnqueueTrigger>::enqueue_trigger(*output, trigger);
                Ok(())
            });
        })?;

        Ok(aud)
    }

    fn apply_ret(&mut self, ret: Option<GameOutcome>) -> () {
        use spru::interactor::SetGameOutcome as _;

        if let Some(ret) = ret {
            self.set_game_outcome(ret);
        }
    }
}

impl<Ret> Output<Ret> for spru::player::init::Output {
    type RetIn = Ret;

    fn create<'scope, 'env>(
        &'env mut self,
        _lua: &mlua::Lua, 
        scope: &'scope mlua::Scope<'scope, 'env>,
    ) 
        -> mlua::Result<mlua::AnyUserData>
    {
        let aud = scope.create_any_userdata(self, |_register| { })?;

        Ok(aud)
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Ret {
        ret
    }
}

impl<Ret> Output<Ret> for spru::game::init::Output {
    type RetIn = Ret;

    fn create<'scope, 'env>(
        &'env mut self,
        _lua: &mlua::Lua, 
        scope: &'scope mlua::Scope<'scope, 'env>,
    ) 
        -> mlua::Result<mlua::AnyUserData>
    {
        let aud = scope.create_any_userdata(self, |_register| { })?;

        Ok(aud)
    }

    fn apply_ret(&mut self, ret: Self::RetIn) -> Ret {
        ret
    }
}

impl<State, Action, Args, Ret, Context, Output> spru_script::Language<State, Action, Args, Ret, Context, Output> for Lua<State, Action> 
where
    State: 'static,
    Args: crate::IntoLuaMulti,
    // `Root: Clone` required because mlua's setters require &mut (understandably), even though we don't mutate the IdTs themselves
    Context: self::Context,
    Output: self::Output<Ret>,
    Output::RetIn: crate::FromLuaMulti,
{
    /// Execute a script with access to the Game's Root. 
    fn exec<Storage>(
        &self, 
        interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
        script: &str,
        args: Args,
    ) -> Result<Ret, Self::Error> 
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

        let lua = self.cached.lua();

        let registry = crate::Registry::new(self.cached.clone());
        let mut registration = crate::Registration::new();
        <Storage::State as spru_script::Scriptable<Action, crate::Registry>>::register(&registry, &mut registration)?;

        let (get_mapping_fn, set_mapping_fn, method_mapping_fn, create_mapping_fn) = registration.into_mapping_fns();

        let ledger = crate::Ledger::new(ledger, &get_mapping_fn, &set_mapping_fn, &method_mapping_fn, &create_mapping_fn);

        
        let mut context_table = lua.create_table()?;

        let ret_in: Output::RetIn = lua.scope(|scope| {
            let ledger_userdata = scope.create_userdata(ledger)?;
            lua.globals().set(crate::key::GLOBAL_LEDGER, ledger_userdata)?;

            context.fill_table(lua, scope, &mut context_table)?;
            lua.globals().set(crate::key::GLOBAL_CONTEXT, context_table)?;

            let lua_output = Output::create(output, lua, scope)?;
            lua.globals().set(crate::key::GLOBAL_OUTPUT, lua_output)?;

            let r = lua
                .load(script)
                .call::<mlua::MultiValue>(crate::IntoLuaMulti::into_lua_multi(args, lua)?)?;

            crate::FromLuaMulti::from_lua_multi(r, lua)
        })?;

        let ret = output.apply_ret(ret_in);
        
        Ok(ret)
    }
}