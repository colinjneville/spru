pub(crate) type MappingFn<Storage, Action> = 
    fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // Box<dyn Fn(&T) -> AnyUserData>
        mlua::AnyUserData
    ) -> mlua::Result<mlua::Value>;

pub(crate) type MappingRoutingFn<'s, Storage, Action> = 
    Box<dyn Fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // Box<dyn Fn(&T) -> AnyUserData>
        mlua::AnyUserData
    ) -> mlua::Result<mlua::Value> + 's>;

pub(crate) type GetterFn<T> = Box<dyn Fn(&mlua::Lua, &T) -> mlua::Result<mlua::Value>>;

pub(crate) type SetterFn<T> = Box<dyn Fn(&mlua::Lua, &T, &T) -> mlua::Result<()>>;