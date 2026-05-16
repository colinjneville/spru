#[derive(Clone, Debug, thiserror::Error)]
#[error("The Rotating index ({index}/{len}) is invalid")]
pub struct IndexOutOfRange {
    pub index: usize,
    pub len: usize,
}

impl IndexOutOfRange {
    pub(crate) fn new(index: usize, len: usize) -> Self {
        Self {
            index,
            len,
        }
    }
}

