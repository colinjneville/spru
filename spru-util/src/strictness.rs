#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Strictness {
    #[default]
    BestEffort,
    AllOrError,
}