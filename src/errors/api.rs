use std::fmt;

#[derive(Debug, Clone)]
pub struct ApiError{
    error: String,
}


impl ApiError {
    pub fn new(error: String) -> Self {
        return ApiError {error};
    }
}


impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}