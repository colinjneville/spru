use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct PlayerInit<Root, Language, Args> {
    language: Language,
    script: String,
    remove_script: Option<String>,
    _p: PhantomData<(Root, Args)>,
}

impl<Root, Language, Args> PlayerInit<Root, Language, Args> {
    pub fn new(language: Language, script: String) -> Self {
        Self {
            language,
            script,
            remove_script: None,
            _p: PhantomData,
        }
    }

    pub fn with_remove_player(mut self, remove_player_script: String) -> Self {
        self.remove_script = Some(remove_player_script);
        self
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
    Language: 
        for<'r> crate::DialectExec<
            (), 
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
    
    fn remove(&self, interactor: &mut spru::player::init::Interactor<Self>) -> spru::player::init::Result<()> {
        if let Some(remove_script) = &self.remove_script {
            let () = self.language.exec(interactor, remove_script, ())?;
            Ok(())
        } else {
            Err(spru::common::error::AnyError::from_string("Removing players is not implemented").into())
        }
    }

    
}