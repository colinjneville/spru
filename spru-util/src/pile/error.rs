#[derive(Clone, Debug)]
#[derive(thiserror::Error)]
pub enum Pop {
    #[error("The pile is empty")]
    Empty,
}

#[derive(Clone, Debug)]
#[derive(thiserror::Error)]
pub enum Insert {
    #[error("The index ({0}/{1}) is invalid")]
    Index(usize, usize),
}

#[derive(Clone, Debug)]
#[derive(thiserror::Error)]
pub enum Remove {
    #[error("The index ({0}/{1}) is invalid")]
    Index(usize, usize),
}