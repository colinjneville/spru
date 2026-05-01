pub(crate) type GetMappingFn<Storage, Action> = 
    fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // GetFn<T> | Nil (existence check)
        mlua::Value
    ) -> mlua::Result<mlua::Value>;

pub(crate) type GetMappingRoutingFn<'s, Storage, Action> = 
    Box<dyn Fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // GetFn<T> | Nil (existence check)
        mlua::Value
    ) -> mlua::Result<mlua::Value> + 's>;

pub(crate) type SetMappingFn<Storage, Action> = 
    fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // SetFn<Action, T>
        mlua::AnyUserData,
        // U
        mlua::Value,
    ) -> mlua::Result<(spru::item::Id, Vec<Action>)>;

pub(crate) type SetMappingRoutingFn<'s, Storage, Action> = 
    Box<dyn Fn(
        &mlua::Lua,
        &mut spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // SetFn<Action, T>
        mlua::AnyUserData,
        // U
        mlua::Value,
    ) -> mlua::Result<()> + 's>;

pub(crate) type MethodMappingFn<Storage, Action> = 
    fn(
        &mlua::Lua,
        &spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // MethodFn<Action, T>
        mlua::AnyUserData,
        // Args
        mlua::MultiValue,
        // (Ret, Id, Actions)
    ) -> mlua::Result<(mlua::MultiValue, spru::item::Id, Vec<Action>)>;

pub(crate) type MethodMappingRoutingFn<'s, Storage, Action> = 
    Box<dyn Fn(
        &mlua::Lua,
        &mut spru::interactor::Ledger<Storage, Action>,
        // IdT
        mlua::AnyUserData, 
        // MethodFn<Action, T>
        mlua::AnyUserData,
        // Args
        mlua::MultiValue,
        // Ret
    ) -> mlua::Result<mlua::MultiValue> + 's>;

// pub(crate) type CreateMappingFn<Storage, Action> = 
//     fn(
//         &mlua::Lua,
//         &spru::interactor::Ledger<Storage, Action>,
//         // CreateFn<Action>
//         mlua::AnyUserData,
//         // Args
//         mlua::MultiValue,
//     ) -> mlua::Result<(Action, CreateIdMappingFn)>;

pub(crate) type CreateMappingRoutingFn<'s, Storage, Action> = 
    Box<dyn Fn(
        &mlua::Lua,
        &mut spru::interactor::Ledger<Storage, Action>,
        // CreateFn<Action, T>
        mlua::AnyUserData,
        // Args
        mlua::MultiValue,
        // IdT
    ) -> mlua::Result<mlua::AnyUserData> + 's>;

pub(crate) type GetFn<T> = Box<dyn Fn(&mlua::Lua, &T) -> mlua::Result<mlua::Value> + Send>;

pub(crate) type SetFn<Action, T> = Box<dyn Fn(&mlua::Lua, &T, mlua::Value) -> mlua::Result<Vec<Action>> + Send>;

pub(crate) type MethodFn<Action, T> = Box<dyn Fn(&mlua::Lua, &T, mlua::MultiValue) -> mlua::Result<(mlua::MultiValue, Vec<Action>)> + Send>;

pub(crate) type CreateFn<Action> = Box<dyn Fn(&mlua::Lua, mlua::MultiValue) -> mlua::Result<(CreateIdFn, Action)> + Send>;

pub(crate) type CreateIdFn = 
    fn(
        &mlua::Lua,
        spru::item::Id,
    ) -> mlua::Result<mlua::AnyUserData>;