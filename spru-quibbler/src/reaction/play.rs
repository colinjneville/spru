use spru::item::IdT;

impl spru::Reaction<crate::State, crate::Actions, IdT<crate::game::Root>> for crate::interaction::Play {
    type Input = crate::interaction::Play;
    type GameOutcome = crate::game::Outcome;

    fn apply(&self, interactor: &mut super::Interactor, input: Self::Input) 
        -> Result<Option<Self::GameOutcome>, spru::reaction::Error>
    {
        todo!()
    }
}
