use std::marker::PhantomData;

use spru_script::Language as _;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Interaction<State, Action, Root, Trigger> {
    script: String,
    #[serde(default, skip)]
    #[serde(bound(deserialize = "State: 'static, Action: 'static"))]
    lua: crate::Lua<State, Action>,
    _p: PhantomData<(State, Action, Root, Trigger)>,
}

impl<State: 'static, Action: 'static, Root, Trigger> Interaction<State, Action, Root, Trigger> {
    pub fn new(script: String) -> Self {
        Self {
            script,
            lua: crate::Lua::new(),
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Trigger> spru::Interaction for Interaction<State, Action, Root, Trigger>
where 
    State: spru_script::ScriptableState<Action, super::Registry>,
    Action: spru::Action<State = State> + 'static,
    Root: super::IntoLua + Clone + 'static,
    Trigger: mlua::FromLua,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type Trigger = Trigger;

    fn apply<'l, 'r, Storage>(&self, interactor: &mut spru::interaction::Interactor<'l, 'r, Storage, Self>) 
        -> spru::interaction::Result<()>
    where 
        Storage: spru::item::Storage<State = Self::State>
    {
        let output: mlua::Value = self.lua.exec(interactor, &self.script)
            .map_err(spru::interaction::Error::from)?;

        println!("{output:?}");

        Ok(())
    }
}