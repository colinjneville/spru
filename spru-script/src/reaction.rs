use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language)]
pub struct Reaction<Root, Trigger, GameOutcome, Language> {
    language: Language,
    script: String,
    _p: PhantomData<(Root, Trigger, GameOutcome)>,
}

impl<Root, Trigger, GameOutcome, Language> Reaction<Root, Trigger, GameOutcome, Language> {
    pub fn new(language: Language, script: String) -> Self {
        Self {
            language,
            script,
            _p: PhantomData,
        }
    }
}

impl<Root, Trigger, GameOutcome, Language> spru::Reaction for Reaction<Root, Trigger, GameOutcome, Language> 
where 
    Language: 
        for<'r> crate::LanguageExec<
            Trigger, 
            (),
            spru::reaction::Context<'r, Root>, 
            spru::reaction::Output<Trigger, GameOutcome>, 
            Error: std::error::Error + Send + Sync + 'static,
        > +
    ,
    Trigger: Clone,
{
    type Action = <Language as crate::Language>::Action;
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