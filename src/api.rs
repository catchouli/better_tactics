mod tactics;
mod user;

use axum::Router;
use axum::extract::{State, Json, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::Value;

use crate::rating::GameResult;
use crate::route::{AppState, ControllerError};
use crate::services::ServiceError;
use crate::services::user_service::UserService;
use crate::srs::{Difficulty, Card};
use crate::time::LocalTimeProvider;
use crate::util;

use crate::controllers::puzzle;

/// API routes.
pub fn routes(app_state: AppState) -> Router {
    Router::new()
        // Tactics.
        .route("/tactics/review", axum::routing::post(tactics::review))

        // User.
        .route("/user/stats", axum::routing::get(user::stats))
        .route("/user/review_forecast/:length_days", axum::routing::get(user::review_forecast))
        .route("/user/rating_history", axum::routing::get(user::rating_history))
        .route("/user/review_score_histogram/:bucket_size", axum::routing::get(user::review_score_histogram))

        .with_state(app_state)
}

/// Type for API responses.
pub struct ApiResponse {
    description: String,
}

impl ApiResponse {
    pub fn ok() -> Self {
        Self {
            description: "OK".into(),
        }
    }
}

/// TODO: return json.
impl IntoResponse for ApiResponse {
    fn into_response(self) -> askama_axum::Response {
        (StatusCode::OK, self.description).into_response()
    }
}

/// Type for API errors.
pub enum ApiError {
    NotFound(String),
    InternalError(String),
    InvalidParameter(String),
}

#[derive(serde::Serialize)]
struct ApiErrorResponse {
    description: String,
}

/// TODO: return json.
impl IntoResponse for ApiError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            Self::NotFound(resource) => (
                StatusCode::NOT_FOUND,
                format!("Not found: {resource}"),
            ),
            Self::InternalError(desc) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {desc}"),
            ),
            Self::InvalidParameter(param) => (
                StatusCode::BAD_REQUEST,
                format!("Bad request: invalid parameter {param}"),
            ),
        }.into_response()
    }
}

// Convert Service errors to Api errors. Generally unhandled service errors should be considered
// internal errors.
impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::InternalError(desc)
                => Self::InternalError(format!("Service error: {desc}")),
            ServiceError::InvalidParameter(param)
                => Self::InternalError(format!("Invalid service parameter: {param}")),
        }
    }
}
