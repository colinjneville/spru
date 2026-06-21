// TODO It should be possible to have a unified language-agnostic set of interactions

use std::marker::PhantomData;

use derive_where::derive_where;

#[derive_where(Debug, Clone, Serialize, Deserialize; Language, Args)]
pub struct Interaction<Root, Trigger, Language, Args> {
    language: Language,
    script: String,
    args: Args,
    _p: PhantomData<(Root, Trigger)>,
}

impl<Root, Trigger, Language, Args> Interaction<Root, Trigger, Language, Args> {
    pub fn new(language: Language, script: String, args: Args) -> Self {
        Self {
            language,
            script,
            args,
            _p: PhantomData,
        }
    }
}

impl<Root, Trigger, Language, Args> spru::Interaction for Interaction<Root, Trigger, Language, Args> 
where 
    Language: 
        for<'r> crate::LanguageExec<
            Args, 
            (),
            spru::interaction::Context<'r, Root>, 
            spru::interaction::Output<Trigger>, 
            Error: std::error::Error + Send + Sync + 'static
        > +
    ,
    Args: Clone,
{
    type Action = <Language as crate::Language>::Action;
    type Root = Root;
    type Trigger = Trigger;

    fn apply<'l, 'r, Storage>(&self, interactor: &mut spru::interaction::Interactor<'l, 'r, Storage, Self>) 
        -> spru::interaction::Result<()>
    where 
        Storage: spru::item::Storage<State = <Self::Action as spru::Action>::State>,
    {
        let () = self.language.exec(interactor, &self.script, self.args.clone())?;

        Ok(())
    }
}