use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Interaction<State, Action, Root, Trigger> {
    script: super::Script,
    _p: PhantomData<(State, Action, Root, Trigger)>,
}

impl<State, Action, Root, Trigger> Interaction<State, Action, Root, Trigger> {
    pub fn new(script: super::Script) -> Self {
        Self {
            script,
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Trigger> spru::Interaction for Interaction<State, Action, Root, Trigger>
where 
    State: crate::scripting::ScriptableState<Action, super::Registry>,
    Action: spru::Action<State = State> + 'static,
    Root: super::IntoLua + 'static,
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
        let output: mlua::Value = self.script.exec(interactor)
            .map_err(spru::interaction::Error::from)?;

        println!("{output:?}");

        Ok(())
    }
}