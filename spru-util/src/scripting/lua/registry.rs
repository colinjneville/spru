use mlua::ObjectLike as _;
use spru::item::IdT;

use crate::scripting::{self, lua};

pub(crate) struct Registry {
    lua: mlua::Lua,
}

impl Registry {
    pub(crate) fn new(lua: mlua::Lua) -> Self {
        Self {
            lua,
        }
    }
}

impl<State, Action> scripting::Registry<State, Action> for Registry {
    type TypeRegistration<'r, Storage> = lua::Registration<Storage, Action>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type MemberRegistration<'r, Storage, T: 'static> = lua::RegistrationType<'r, T>
    where
        Storage: spru::item::Storage<State = State>,
    ;

    type Error = mlua::Error;
}

impl<State, Action, T> scripting::RegistryType<State, Action, T> for Registry
where
    State: spru::State,
    Action: spru::Action,
    T: scripting::ScriptableType<State, Action, Self>,
    T: spru::item::storage::Storable<State> + 'static,
{
    fn register_type<Storage>(&self, registration: &mut lua::Registration<Storage, Action>) 
        -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
        // T: ScriptableType<State, Action, Self> + 'static,
    {
        self.lua.register_userdata_type::<IdT<T>>(|mlua_registry| {
            let mut registration = lua::RegistrationType::new(mlua_registry);

            // mlua doesn't let us passthrough these errors
            if let Err(err) = T::register::<Storage>(self, &mut registration) {
                panic!("{}", err);
            }
        })?;

        registration.register_mapping::<T>(
            |
                lua: &mlua::Lua, 
                ledger: &spru::interactor::Ledger<Storage, Action>, 
                id: mlua::AnyUserData, 
                mapping: mlua::AnyUserData
            | {
                let id = *id.borrow()?;
                let mapping = mapping.borrow::<lua::func::GetterFn<T>>()?;

                match ledger.get::<T>(id) {
                    Ok(existing) => {
                        mapping(lua, &*existing)
                    }
                    Err(e) => {
                        Err(mlua::Error::runtime(&format!("Item '{id:?}' ({}) not found: {e}", std::any::type_name::<T>())))
                    }
                }    
            }
        );
        
        Ok(())
    }
}


impl<State, Action, T: 'static, U> scripting::RegistryGetter<State, Action, T, U> for Registry
where 
    State: spru::State,
    Action: spru::Action,
    T: spru::item::storage::Storable<State>,
    U: lua::IntoLua + 'static,
{
    fn register_get<Storage>(&self, registration: &mut lua::RegistrationType<'_, T>, ident: &str, getter: fn(&T) -> U)
         -> Result<(), Self::Error> 
    where
        Storage: spru::item::Storage<State = State>,
    {
        let f = move |lua: &mlua::Lua, this: &IdT<T>| {
            let id = lua::IntoLua::into_lua(*this, lua)?;

            let ledger = lua.globals()
                .get::<mlua::AnyUserData>(lua::key::LEDGER_GLOBAL)
                .expect("Ledger not registered");

            let getter_clone = getter.clone();
            let lua_getter: lua::func::GetterFn<T> = Box::new(move |lua, t| {
                getter_clone(t).into_lua(lua)
            });
            let lua_getter = lua.create_any_userdata(lua_getter)?;

            let output = ledger.call_method::<mlua::Value>(lua::key::LEDGER_METHOD_GET, (id, lua_getter))?;

            Ok(output)
        };

        registration.add_getter(ident, f);
        Ok(())
    }
}



// impl<'r, Storage, Action, T, U> ScriptableSetter<U> for LuaTypeRegistry<'r, Storage, Action, T>
// where 
//     Action: spru::Action,
// //     U: mlua::UserData,
// {
//     fn register_set(&mut self, ident: &str, setter: fn(&Self::T, U) -> Vec<Self::Action>) -> Result<(), Self::Error> {
//         todo!()
//         // use mlua::UserDataFields as _;

//         // self.registry.add_field_function_set(ident, setter);
//         // Ok(())
//     }
// }

// struct LuaGetter<T>(GetterFn<T>);

// impl<T> LuaGetter<T> {
//     fn new(f: GetterFn<T>) -> Self {
//         Self(f)
//     }
// }

// impl<T> mlua::UserData for LuaGetter<T> { }