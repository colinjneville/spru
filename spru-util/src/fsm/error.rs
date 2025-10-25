#[derive(Debug, Clone, thiserror::Error)]
pub enum Transition {
    #[error("Input is not allowed for this state.")]
    TransitionImpossible(#[from] rust_fsm::TransitionImpossibleError),
}
