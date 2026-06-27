use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct GameInit<Root, Language, Args> {
    language: Language,
    script: String,
    args: Args,
    _p: PhantomData<(Root, )>,
}

impl<Root, Language, Args> GameInit<Root, Language, Args> {
    pub fn new(language: Language, script: String, args: Args) -> Self {
        Self {
            language,
            script,
            args,
            _p: PhantomData,
        }
    }
}

impl<Root, Language, Args> spru::game::Init for GameInit<Root, Language, Args> 
where 
    Language: crate::DialectExec<
        Args,
        Root,
        spru::game::init::Context, 
        spru::game::init::Output,

        Error: std::error::Error + Send + Sync + 'static,
    >,
{
    type Root = Root;
    type Action = <Language as crate::Dialect>::Action;

    fn initialize(self, interactor: &mut spru::game::init::Interactor<Self>) -> spru::game::init::Result<Self::Root> {
        let root = self.language.exec(interactor, &self.script, self.args)?;

        Ok(root)
    }
}