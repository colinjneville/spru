use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct PlayerInit<Root, Language, Args> {
    language: Language,
    script: String,
    _p: PhantomData<(Root, Args)>,
}

impl<Root, Language, Args> PlayerInit<Root, Language, Args> {
    pub fn new(language: Language, script: String) -> Self {
        Self {
            language,
            script,
            _p: PhantomData,
        }
    }
}

impl<Root, Language, Args> spru::player::Init for PlayerInit<Root, Language, Args> 
where 
    Language: 
        for<'r> crate::DialectExec<
            Args, 
            (),
            spru::player::init::Context<'r, Root>, 
            spru::player::init::Output,
            Error: std::error::Error + Send + Sync + 'static,
        >,
    Args: Clone,
{
    type In = Args;
    type Root = Root;
    type Action = <Language as crate::Dialect>::Action;

    fn initialize(&self, interactor: &mut spru::player::init::Interactor<Self>, input: Self::In) -> spru::player::init::Result<()> {
        let () = self.language.exec(interactor, &self.script, input)?;

        Ok(())
    }
}