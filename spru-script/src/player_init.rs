use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct PlayerInit<State, Action, Root, Language, Args> {
    language: Language,
    script: String,
    _p: PhantomData<(State, Action, Root, Args)>,
}

impl<State, Action, Root, Language, Args> PlayerInit<State, Action, Root, Language, Args> {
    pub fn new(language: Language, script: String) -> Self {
        Self {
            language,
            script,
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Language, Args> spru::player::Init for PlayerInit<State, Action, Root, Language, Args> 
where 
    State: crate::Scriptable<Action, Language::Registry>,
    Action: spru::Action<State = State>,
    Language: 
        crate::LanguageBase<State, Action, Error: std::error::Error + Send + Sync + 'static> +
        for<'r> crate::Language<
            State, 
            Action, 
            Args, 
            (),
            spru::player::init::Context<'r, Root>, 
            spru::player::init::Output,
        >,
    Args: Clone,
{
    type In = Args;
    type Root = Root;
    type State = State;
    type Action = Action;

    fn initialize(&self, interactor: &mut spru::player::init::Interactor<Self>, input: Self::In) -> spru::player::init::Result<()> {
        let () = self.language.exec(interactor, &self.script, input)?;

        Ok(())
    }
}