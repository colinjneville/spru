#[derive(Clone, Debug)]
#[derive(thiserror::Error)]
pub enum Pop {
    #[error("The pile is empty")]
    Empty,
}