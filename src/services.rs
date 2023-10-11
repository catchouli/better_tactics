pub mod user_service;
pub mod tactics_service;

use crate::db::DatabaseError;

/// Type for service results.
type ServiceResult<T> = Result<T, ServiceError>;

/// Type for service errors.
#[derive(Debug)]
pub enum ServiceError {
    InternalError(String),
    InvalidParameter(String),
}

impl From<String> for ServiceError {
    fn from(err: String) -> Self {
        Self::InternalError(err.to_string())
    }
}

impl From<DatabaseError> for ServiceError {
    fn from(err: DatabaseError) -> Self {
        Self::InternalError(err.to_string())
    }
}
