// TODO It should be possible to have a unified language-agnostic set of interactions

use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct Interaction<State, Action, Root, Trigger, Language, Args> {
    language: Language,
    script: String,
    args: Args,
    _p: PhantomData<(State, Action, Root, Trigger)>,
}

impl<State, Action, Root, Trigger, Language, Args> Interaction<State, Action, Root, Trigger, Language, Args> {
    pub fn new(language: Language, script: String, args: Args) -> Self {
        Self {
            language,
            script,
            args,
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Trigger, Language, Args> spru::Interaction for Interaction<State, Action, Root, Trigger, Language, Args> 
where 
    State: crate::Scriptable<Action, Language::Registry>,
    Action: spru::Action<State = State> + 'static,
    // Root: super::IntoLua + Clone + 'static,
    // Trigger: mlua::FromLua,
    Language: 
        crate::LanguageBase<State, Action> +
        for<'r> crate::Language<
            State, 
            Action, 
            Args, 
            spru::interaction::Context<'r, Root>, 
            spru::interaction::Output<Trigger>, 
            Error: std::error::Error + Send + Sync + 'static
        > +
    ,
    Args: Clone,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type Trigger = Trigger;

    fn apply<'l, 'r, Storage>(&self, interactor: &mut spru::interaction::Interactor<'l, 'r, Storage, Self>) 
        -> spru::interaction::Result<()>
    where 
        Storage: spru::item::Storage<State = Self::State>,
    {
        let () = self.language.exec(interactor, &self.script, self.args.clone())?;

        Ok(())
    }
}