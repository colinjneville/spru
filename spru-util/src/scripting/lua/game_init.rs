use std::marker::PhantomData;

use crate::scripting;


#[derive(Debug, Clone)]
pub struct GameInit<Root, State, Action> {
    script: super::Script,
    _p: PhantomData<(Root, State, Action)>,
}

impl<Root, State, Action> GameInit<Root, State, Action> {
    pub fn new(script: super::Script) -> Self {
        Self {
            script,
            _p: PhantomData,
        }
    }
}

impl<Root, State, Action> spru::game::Init for GameInit<Root, State, Action> 
where
    Root: mlua::FromLua,
    State: scripting::ScriptableState<Action, super::Registry>,
    Action: spru::Action<State = State>,
{
    type Root = Root;
    type State = State;
    type Action = Action;

    fn initialize(self, interactor: &mut spru::game::init::Interactor<Self>) -> spru::game::init::Result<Self::Root> {
        Ok(self.script.exec_no_root(interactor)?)
    }
}