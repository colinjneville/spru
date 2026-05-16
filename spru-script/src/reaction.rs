use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language)]
pub struct Reaction<State, Action, Root, Trigger, GameOutcome, Language> {
    language: Language,
    script: String,
    _p: PhantomData<(State, Action, Root, Trigger, GameOutcome)>,
}

impl<State, Action, Root, Trigger, GameOutcome, Language> Reaction<State, Action, Root, Trigger, GameOutcome, Language> {
    pub fn new(language: Language, script: String) -> Self {
        Self {
            language,
            script,
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Trigger, GameOutcome, Language> spru::Reaction for Reaction<State, Action, Root, Trigger, GameOutcome, Language> 
where 
    State: crate::Scriptable<Action, Language::Registry>,
    Action: spru::Action<State = State> + 'static,
    Language: crate::LanguageBase<State, Action> +
        for<'r> crate::Language<
            State, 
            Action, 
            Trigger, 
            (),
            spru::reaction::Context<'r, Root>, 
            spru::reaction::Output<Trigger, GameOutcome>, 
            Error: std::error::Error + Send + Sync + 'static
        > +
    ,
    Trigger: Clone,
{
    type State = State;
    type Action = Action;
    type Root = Root;
    type Trigger = Trigger;
    type GameOutcome = GameOutcome;

    fn apply<'l, 'r>(
        &self, 
        interactor: &mut spru::reaction::Interactor<'l, 'r, Self>, 
        trigger: Self::Trigger,
    ) 
        -> spru::action::Result<()> 
    {
        let () = self.language.exec(interactor, &self.script, trigger)?;

        Ok(())
    }
}