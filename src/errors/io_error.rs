use std::fmt;

#[derive(Debug, Clone)]
pub struct IoError {
    error: String,
}

impl IoError {
    pub fn new(error: String) -> Self {
        return IoError { error };
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}
