use std::error::Error;
use std::fmt::Display;

use sqlx::migrate::MigrateError;

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
    ConnectionError(ErrorDetails),
    QueryError(ErrorDetails),
    ParsingError(ErrorDetails),
    MigrationError(ErrorDetails),
    BackupError(ErrorDetails),
}

unsafe impl Send for DatabaseError {}

impl DatabaseError {
    fn details(&self) -> &ErrorDetails {
        match self {
            DatabaseError::ConnectionError(details) => details,
            DatabaseError::QueryError(details) => details,
            DatabaseError::ParsingError(details) => details,
            DatabaseError::MigrationError(details) => details,
            DatabaseError::BackupError(details) => details,
        }
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionError(details)
                => write!(f, "{} connection error: {}", details.backend, details.description),
            DatabaseError::QueryError(details)
                => write!(f, "{} query execution error: {}", details.backend, details.description),
            DatabaseError::ParsingError(details)
                => write!(f, "{} parsing error: {}", details.backend, details.description),
            DatabaseError::MigrationError(details)
                => write!(f, "{} migration error: {}", details.backend, details.description),
            DatabaseError::BackupError(details)
                => write!(f, "{} backup error: {}", details.backend, details.description),
        }
    }
}

impl Error for DatabaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.details().source.as_ref().map(AsRef::as_ref)
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(e: sqlx::Error) -> Self {
        DatabaseError::QueryError(ErrorDetails {
            backend: "sqlite".to_string(),
            description: format!("Database query error: {e}"),
            source: Some(e.into())
        })
    }
}

impl From<chrono::ParseError> for DatabaseError {
    fn from(e: chrono::ParseError) -> Self {
        DatabaseError::ParsingError(ErrorDetails {
            backend: "chrono".to_string(),
            description: format!("{e}"),
            source: Some(e.into())
        })
    }
}

impl From<MigrateError> for DatabaseError {
    fn from(e: MigrateError) -> Self {
        DatabaseError::MigrationError(ErrorDetails {
            backend: "sqlx".to_string(),
            description: format!("{e}"),
            source: Some(e.into()),
        })
    }
}
