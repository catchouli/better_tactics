use std::error::Error;
use std::fmt::Display;

use crate::route::InternalError;

/// A result type that boxes errors to a Box<dyn Error>.
pub type DbResult<T> = Result<T, DatabaseError>;

/// A struct containing error details.
#[derive(Debug)]
pub struct ErrorDetails {
    pub backend: String,
    pub description: String,
    pub source: Option<Box<dyn Error>>,
}

/// An error type indicating a database error.
#[derive(Debug)]
pub enum DatabaseError {
    ParameterError(ErrorDetails),
    ConnectionError(ErrorDetails),
    QueryError(ErrorDetails),
    ParsingError(ErrorDetails),
    InternalError(ErrorDetails),
}

unsafe impl Send for DatabaseError {}

impl DatabaseError {
    fn details(&self) -> &ErrorDetails {
        match self {
            DatabaseError::ParameterError(details) => details,
            DatabaseError::ConnectionError(details) => details,
            DatabaseError::QueryError(details) => details,
            DatabaseError::ParsingError(details) => details,
            DatabaseError::InternalError(details) => details,
        }
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ParameterError(details)
                => write!(f, "Invalid parameter {}", details.description),
            DatabaseError::ConnectionError(details)
                => write!(f, "{} connection error: {}", details.backend, details.description),
            DatabaseError::QueryError(details)
                => write!(f, "{} query execution error: {}", details.backend, details.description),
            DatabaseError::ParsingError(details)
                => write!(f, "{} parsing error: {}", details.backend, details.description),
            DatabaseError::InternalError(details)
                => write!(f, "{} internal error: {}", details.backend, details.description),
        }
    }
}

impl Error for DatabaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.details().source.as_ref().map(AsRef::as_ref)
    }
}

impl From<DatabaseError> for InternalError {
    fn from(value: DatabaseError) -> Self {
        Self {
            description: value.to_string(),
        }
    }
}
