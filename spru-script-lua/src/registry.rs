use mlua::ObjectLike as _;
use spru::item::IdT;

pub struct Registry {
    cached: crate::instance::Cached,
}

impl Registry {
    pub(crate) fn new(cached: crate::instance::Cached) -> Self {
        Self {
            cached,
        }
    }

    fn lua(&self) -> &mlua::Lua {
        self.cached.lua()
    }
}

impl<State, Action> spru_script::Registry<State, Action> for Registry {
    type TypeRegistration<'r, Storage> = crate::Registration<Storage, Action>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type MemberRegistration<'r, Storage, T: 'static> = crate::RegistrationType<'r, T>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type Error = mlua::Error;
}

impl<State, Action, T> spru_script::RegistryType<State, Action, T> for Registry
where
    State: spru::State,
    Action: spru::Action,
    T: spru_script::ScriptableType<State, Action, Self>,
    T: spru::item::storage::Storable<State> + 'static,
{
    fn register_type<Storage>(
        &self, 
        registration: &mut crate::Registration<Storage, Action>,
        type_path: Option<spru_script::ScriptablePath>,
    ) 
        -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
        // T: ScriptableType<State, Action, Self> + 'static,
    {
        self.lua().register_userdata_type::<IdT<T>>(|mlua_registry| {
            use mlua::UserDataMethods as _;

            // mlua doesn't let us passthrough errors, so they are panicking

            // Special `exists` method
            mlua_registry.add_method("exists", |lua, this, ()| {
                let id = crate::IntoLua::into_lua(*this, lua)?;

                let ledger = lua.globals()
                    .get::<mlua::AnyUserData>(crate::key::LEDGER_GLOBAL)
                    .expect("Ledger not registered");

                // We overload `get` to check for existance by sending a nil `mapping`
                let exists = ledger.call_method::<bool>(crate::key::LEDGER_METHOD_GET, (id, mlua::Value::Nil))?;

                Ok(exists)
            });
            
            let static_table = if let Some(type_path) = type_path {
                fn register_path(lua: &mlua::Lua, type_parameters_metatable: &mlua::Table, scriptable_path: &spru_script::ScriptablePath) -> mlua::Result<mlua::Table> {
                    let &spru_script::ScriptablePath(path, args) = scriptable_path;
                    let mut keys = vec![];
                    for &segment in path {
                        keys.push(mlua::IntoLua::into_lua(segment, lua)?);
                    }

                    if !args.is_empty() {
                        let args_table = lua.create_table()?;

                        for arg in args {
                            let arg_table = register_path(lua, type_parameters_metatable, arg)?;
                            args_table.push(arg_table)?;
                        }

                        let key = mlua::IntoLua::into_lua(args_table, lua)?;
                    
                        keys.push(key);
                    }

                    let mut current_table = lua.globals();
                    for key in keys {
                        if key.is_table() {
                            // We've reached type parameters: unlike path strings, tables containing the same tables
                            // won't index by default - we need a special __index metafunction that compares the keys
                            // structurally
                            current_table.set_metatable(Some(type_parameters_metatable.clone()))?;
                        }
                        
                        current_table = if let Ok(table) = current_table.get::<mlua::Table>(&key) {
                            table
                        } else {
                            let table = lua.create_table()?;
                            current_table.set(&key, table)?;
                            current_table.get::<mlua::Table>(&key)?
                        };
                    }

                    Ok(current_table)
                }

                let static_table = register_path(self.lua(), self.cached.type_parameters_metatable(), &type_path)
                    .expect("Failed to register type path");
                Some(static_table)
            } else {
                None
            };

            let mut registration = crate::RegistrationType::new(self.lua(), mlua_registry, static_table)
                .expect("Failed to register type");

            if let Err(err) = T::register::<Storage>(self, &mut registration) {
                panic!("{err}");
            }
        })?;

        registration.register_mappings::<T>(
            |
                lua: &mlua::Lua, 
                ledger: &spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::Value,
            | {
                let id = *id.borrow()?;

                match ledger.get::<T>(id) {
                    Ok(existing) => {
                        if let Some(mapping) = mapping.as_userdata() {
                            let mapping = mapping.borrow::<crate::func::GetFn<T>>()?;
                            mapping(lua, &*existing)
                        } else if mapping.is_nil() {
                            Ok(mlua::Value::Boolean(true))
                        } else {
                            panic!("expected mapping function or nil");
                        }
                    }
                    Err(e) => {
                        // We overload `get` to check for existance by sending a nil `mapping`
                        if mapping.is_nil() {
                            Ok(mlua::Value::Boolean(false))
                        } else {
                            Err(mlua::Error::runtime(&format!("Item '{id:?}' ({}) not found: {e}", std::any::type_name::<T>())))
                        }
                    }
                }    
            },

            |
                lua: &mlua::Lua, 
                ledger: &spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::AnyUserData,
                value: mlua::Value,
            | {
                let id = *id.borrow()?;
                let mapping = mapping.borrow::<crate::func::SetFn<Action, T>>()?;

                match ledger.get::<T>(id) {
                    Ok(existing) => {
                        let actions = mapping(lua, &*existing, value)?;
                        
                        Ok((id.untyped(), actions))
                    }
                    Err(e) => {
                        Err(mlua::Error::runtime(&format!("Item '{id:?}' ({}) not found: {e}", std::any::type_name::<T>())))
                    }
                }    
            },

            |
                lua: &mlua::Lua, 
                ledger: &spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::AnyUserData,
                args: mlua::MultiValue,
            | {
                let id = *id.borrow()?;
                let mapping = mapping.borrow::<crate::func::MethodFn<Action, T>>()?;

                match ledger.get::<T>(id) {
                    Ok(existing) => {
                        let (ret, actions) = mapping(lua, &*existing, args)?;
                        
                        Ok((ret, id.untyped(), actions))
                    }
                    Err(e) => {
                        Err(mlua::Error::runtime(&format!("Item '{id:?}' ({}) not found: {e}", std::any::type_name::<T>())))
                    }
                }    
            },
        );
        
        Ok(())
    }
}


impl<State, Action, T: 'static, U> spru_script::RegistryGetter<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    U: crate::IntoLua + 'static,
{
    fn register_get<Storage>(&self, registration: &mut crate::RegistrationType<'_, T>, ident: &str, getter: fn(&T) -> U)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |lua: &mlua::Lua, this: &IdT<T>| {
            let id = crate::IntoLua::into_lua(*this, lua)?;

            let ledger = lua.globals()
                .get::<mlua::AnyUserData>(crate::key::LEDGER_GLOBAL)
                .expect("Ledger not registered");

            let lua_getter: crate::func::GetFn<T> = Box::new(move |lua, t| {
                getter(t).into_lua(lua)
            });
            let lua_getter = lua.create_any_userdata(lua_getter)?;

            let output = ledger.call_method::<mlua::Value>(crate::key::LEDGER_METHOD_GET, (id, lua_getter))?;

            Ok(output)
        };

        registration.add_getter(ident, f);
        Ok(())
    }
}

impl<State, Action, T: 'static, U> spru_script::RegistrySetter<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    U: crate::FromLua + 'static,
{
    fn register_set<Storage>(&self, registration: &mut crate::RegistrationType<'_, T>, ident: &str, setter: fn(&T, U) -> Vec<Action>)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |lua: &mlua::Lua, this: &mut IdT<T>, value: mlua::Value| {
            let id = crate::IntoLua::into_lua(*this, lua)?;

            let ledger = lua.globals()
                .get::<mlua::AnyUserData>(crate::key::LEDGER_GLOBAL)
                .expect("Ledger not registered");

            let lua_setter: crate::func::SetFn<Action, T> = Box::new(move |lua, t, u| {
                Ok(setter(t, crate::FromLua::from_lua(u, lua)?))
            });
            let lua_setter = lua.create_any_userdata(lua_setter)?;

            ledger.call_method::<mlua::Value>(crate::key::LEDGER_METHOD_SET, (id, lua_setter, value))?;

            Ok(())
        };

        registration.add_setter(ident, f);
        Ok(())
    }
}

impl<State, Action, T: 'static, Args, Ret> spru_script::RegistryMethod<State, Action, T, Args, Ret> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    Args: mlua::FromLuaMulti + 'static,
    Ret: mlua::IntoLuaMulti + 'static,
{
    fn register_method<Storage>(&self, registration: &mut crate::RegistrationType<'_, T>, ident: &str, method: fn(&T, Args) -> (Ret, Vec<Action>))
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |lua: &mlua::Lua, this: &IdT<T>, args: mlua::MultiValue| {
            let id = crate::IntoLua::into_lua(*this, lua)?;

            let ledger = lua.globals()
                .get::<mlua::AnyUserData>(crate::key::LEDGER_GLOBAL)
                .expect("Ledger not registered");

            let lua_method: crate::func::MethodFn<Action, T> = Box::new(move |lua, t, args| {
                let args = mlua::FromLuaMulti::from_lua_multi(args, lua)?;
                let (ret, actions) = method(t, args);
                let ret = mlua::IntoLuaMulti::into_lua_multi(ret, lua)?;
                Ok((ret, actions))
            });
            let lua_method = lua.create_any_userdata(lua_method)?;

            let ret = ledger.call_method::<mlua::MultiValue>(crate::key::LEDGER_METHOD_METHOD, (id, lua_method, args))?;

            Ok(ret)
        };

        registration.add_method(ident, f);
        Ok(())
    }
}


impl<State, Action, Create, T: 'static, Args> spru_script::RegistryCreate<State, Action, Create, T, Args> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    Create: spru::action::Create<T = T> + Into<Action> + 'static,
    T: spru::item::storage::Storable<State>,
    Args: mlua::FromLuaMulti + 'static,
{
    fn register_create<Storage>(&self, registration: &mut crate::RegistrationType<'_, T>, ident: &str, create: fn(Args) -> Create)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |lua: &mlua::Lua, args: mlua::MultiValue| {
            
            let ledger = lua.globals()
                .get::<mlua::AnyUserData>(crate::key::LEDGER_GLOBAL)
                .expect("Ledger not registered");

            let lua_create: crate::func::CreateFn<Action> = Box::new(move |lua, args| {
                let args = mlua::FromLuaMulti::from_lua_multi(args, lua)?;
                let action = create(args);

                let idt_fn: crate::func::CreateIdFn = |lua, id| {
                    let idt = id.force_type::<T>();
                    lua.create_any_userdata(idt)
                };

                Ok((idt_fn, action.into()))
            });

            let lua_create = lua.create_any_userdata(lua_create)?;

            let create = ledger.call_method::<mlua::AnyUserData>(crate::key::LEDGER_METHOD_CREATE, (lua_create, args))?;

            Ok(create)
        };

        registration.add_create(ident, f)?;
        Ok(())
    }
}
