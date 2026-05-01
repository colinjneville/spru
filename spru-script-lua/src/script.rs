use spru_script::{Language as _, LanguageNoRoot as _};

// #[derive(Debug, Clone)]
// pub struct Script {
//     lua: crate::Lua,
//     script: String,
// }

// impl Script {
//     pub(crate) fn new(lua: crate::Lua, script: String) -> Self {
//         Self {
//             lua, 
//             script,
//         }
//     }

//     pub(crate) fn exec<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
//         &self, 
//         interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
//     ) -> mlua::Result<R> 
//     where
//         Storage: spru::item::Storage<State = Action::State>,
//         Storage::State: spru_script::ScriptableState<Action, crate::Registry>,
//         Action: spru::Action,
//         Context: spru::interactor::GetRoot<Root: crate::IntoLua + Clone + 'static>,
//     {
//         self.lua.exec(interactor, &self.script)
//     }

//     pub(crate) fn exec_no_root<Storage, Action, Context, Output, R: mlua::FromLuaMulti>(
//         &self, 
//         interactor: &mut spru::Interactor<'_, Storage, Action, Context, Output>,
//     ) -> mlua::Result<R> 
//     where
//         Storage: spru::item::Storage<State = Action::State>,
//         Storage::State: spru_script::ScriptableState<Action, crate::Registry>,
//         Action: spru::Action,
//     {
//         self.lua.exec_no_root(interactor, &self.script)
//     }
// }