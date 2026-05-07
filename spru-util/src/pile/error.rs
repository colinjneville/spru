#[derive(Clone, Debug, thiserror::Error)]
#[error("The Pile is empty")]
pub struct Empty;

#[derive(Clone, Debug, thiserror::Error)]
#[error("The Pile index ({index}/{len}) is invalid")]
pub struct IndexOutOfRange {
    pub index: usize,
    pub len: usize,
}
