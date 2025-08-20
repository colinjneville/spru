use spru::item::IdT;
use spru_util::verbatim;

impl spru::Reaction<crate::State, crate::Actions, IdT<crate::game::Root>> for crate::game::Start {
    type Input = crate::game::Start;
    type GameOutcome = crate::game::Outcome;

    fn apply(&self, interactor: &mut super::Interactor, input: Self::Input) 
        -> Result<Option<Self::GameOutcome>, spru::reaction::Error>
    {
        let root = interactor.get_root()?;
        let new_root = crate::game::Root {
            has_started: true,
            ..(*root).clone()
        };
        interactor.get_root()?
            .update(verbatim::update(new_root));

        Ok(None)
    }
}