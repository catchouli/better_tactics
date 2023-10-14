mod tactics;
mod user;

use axum::{Router, Json};
use axum::body::Body;
use axum::http::{StatusCode, Request};
use axum::response::{Response, IntoResponse};

use crate::app::AppState;
use crate::services::ServiceError;

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
        .route("/user/reset_rating/:new_rating", axum::routing::get(user::reset_rating))

        .fallback(not_found)

        .with_state(app_state)
}

/// Not found handler.
pub async fn not_found(req: Request<Body>) -> ApiError {
    ApiError::NotFound(req.uri().to_string())
}

/// Type for API responses.
#[derive(serde::Serialize)]
pub struct ApiResponse {
    response: String,
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
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
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(endpoint) => (
                StatusCode::NOT_FOUND,
                Json(ApiErrorResponse {
                    error: format!("No such api endpoint /api{endpoint}"),
                })
            ),
            Self::InternalError(desc) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: format!("Internal server error: {desc}"),
                })
            ),
            Self::InvalidParameter(param) => (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse {
                    error: format!("Bad request: invalid parameter {param}"),
                })
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
        }
    }
}
