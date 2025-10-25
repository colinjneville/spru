#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Strictness {
    #[default]
    BestEffort,
    AllOrError,
}
