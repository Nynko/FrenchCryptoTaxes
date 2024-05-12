use std::fmt;

#[derive(Debug, Clone)]
pub struct MappingError {
    error: String,
}

impl MappingError {
    pub fn new(error: String) -> Self {
        return MappingError { error };
    }
}

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}
