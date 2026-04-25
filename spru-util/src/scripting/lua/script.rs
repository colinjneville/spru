
use crate::scripting;

use super as lua;

#[derive(Debug, Clone)]
pub struct Script {
    lua: lua::Lua,
    script: String,
}

impl Script {
    pub(crate) fn new(lua: lua::Lua, script: String) -> Self {
        Self {
            lua, 
            script,
        }
    }

    pub(crate) fn exec<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
        &self, 
        interactor: &spru::Interactor<'_, Storage, Action, Context, Output>,
    ) -> mlua::Result<R> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: scripting::ScriptableState<Action, lua::Registry>,
        Action: spru::Action + 'static,
        Context: spru::interactor::GetRoot<Root: lua::IntoLua + 'static>,
    {
        self.lua.exec(interactor, &self.script)
    }

    pub(crate) fn exec_no_root<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
        &self, 
        interactor: &spru::Interactor<'_, Storage, Action, Context, Output>,
    ) -> mlua::Result<R> 
    where
        Storage: spru::item::Storage<State = Action::State>,
        Storage::State: scripting::ScriptableState<Action, lua::Registry>,
        Action: spru::Action + 'static,
    {
        self.lua.exec_no_root(interactor, &self.script)
    }
}