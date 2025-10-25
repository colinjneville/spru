#[derive(Clone, Debug, thiserror::Error)]
pub enum Pop {
    #[error("The pile is empty")]
    Empty,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum Insert {
    #[error("The index ({0}/{1}) is invalid")]
    Index(usize, usize),
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum Remove {
    #[error("The index ({0}/{1}) is invalid")]
    Index(usize, usize),
}
