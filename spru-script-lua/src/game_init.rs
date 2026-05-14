// use std::marker::PhantomData;

// use spru_script::LanguageNoRoot as _;


// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
// pub struct GameInit<State, Action, Root> {
//     script: String,
//     #[serde(default, skip)]
//     #[serde(bound(deserialize = "State: 'static, Action: 'static"))]
//     lua: crate::Lua<State, Action>,
//     _p: PhantomData<(Root, )>,
// }

// impl<State: 'static, Action: 'static, Root> GameInit<State, Action, Root> {
//     pub fn new(script: String) -> Self {
//         Self {
//             script,
//             lua: crate::Lua::new(),
//             _p: PhantomData,
//         }
//     }
// }

// impl<State, Action, Root> spru::game::Init for GameInit<State, Action, Root> 
// where
//     State: spru_script::Scriptable<Action, super::Registry>,
//     Action: spru::Action<State = State>,
//     Root: crate::FromLuaMulti,
// {
//     type Root = Root;
//     type State = State;
//     type Action = Action;

//     fn initialize(self, interactor: &mut spru::game::init::Interactor<Self>) -> spru::game::init::Result<Self::Root> {
//         Ok(self.lua.exec_no_root(interactor, &self.script, ())?)
//     }
// }