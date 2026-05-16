use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct GameInit<State, Action, Root, Language, Args> {
    language: Language,
    script: String,
    args: Args,
    _p: PhantomData<(State, Action, Root)>,
}

impl<State, Action, Root, Language, Args> GameInit<State, Action, Root, Language, Args> {
    pub fn new(language: Language, script: String, args: Args) -> Self {
        Self {
            language,
            script,
            args,
            _p: PhantomData,
        }
    }
}

impl<State, Action, Root, Language, Args> spru::game::Init for GameInit<State, Action, Root, Language, Args> 
where 
    State: crate::Scriptable<Action, Language::Registry>,
    Action: spru::Action<State = State>,
    Language: crate::Language<
        State, 
        Action, 
        Args, 
        Root,
        spru::game::init::Context, 
        spru::game::init::Output,
        Error: std::error::Error + Send + Sync + 'static,
    >,
{
    type Root = Root;
    type State = State;
    type Action = Action;

    fn initialize(self, interactor: &mut spru::game::init::Interactor<Self>) -> spru::game::init::Result<Self::Root> {
        let root = self.language.exec(interactor, &self.script, self.args)?;

        Ok(root)
    }
}