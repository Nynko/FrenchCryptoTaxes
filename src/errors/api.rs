use std::fmt;

#[derive(Debug, Clone)]
pub enum ApiError {
    ApiCallError(String),
    MappingError(MappingError),
    CouldNotFindPrice { pairs: Vec<(String, String)> },
    DeserializationError(String),
}

#[derive(Debug, Clone)]
pub enum MappingError {
    Other(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::ApiCallError(error) => write!(f, "{}", *error),
            ApiError::MappingError(error) => match error {
                MappingError::Other(e) => write!(f, "{}", *e),
            },
            ApiError::CouldNotFindPrice { pairs } => {
                let pairs_string = pairs
                    .iter()
                    .map(|(first, second)| format!("{}/{}", first, second))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "Couldn't find price for pairs: {pairs_string} ")
            }
            ApiError::DeserializationError(e) => {
                write!(f, "Error during serde deserialisation: {e} ")
            }
        }
    }
}
