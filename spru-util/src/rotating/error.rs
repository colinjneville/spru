use std::fmt;

#[derive(Debug, Default)]
pub struct Expected;

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Actual value did not match the expected value")
    }
}

impl std::error::Error for Expected {
    
}