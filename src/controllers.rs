use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::http::status::StatusCode;
use axum::response::IntoResponse;

use crate::app::AppState;
use crate::services::ServiceError;

pub mod index;
pub mod puzzle;
pub mod about;

const ASSETS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Our routes.
pub fn routes(app_state: AppState) -> Router {
    Router::new()
        // Basic pages.
        .route("/", axum::routing::get(index::index_page))
        .route("/about", axum::routing::get(about::about_page))

        // Tactics pages.
        .route("/tactics", axum::routing::get(puzzle::next_review))
        .route("/tactics/new", axum::routing::get(puzzle::random_puzzle))
        .route("/tactics/by_id/:puzzle_id", axum::routing::get(puzzle::specific_puzzle))

        .fallback(not_found)

        .with_state(app_state)
}

/// Not found handler.
pub async fn not_found(req: Request<Body>) -> ControllerError {
    ControllerError::NotFound(req.uri().to_string())
}

/// The base template data.
pub struct BaseTemplateData {
    pub assets_version: String,
}

impl Default for BaseTemplateData {
    fn default() -> Self {
        Self {
            assets_version: ASSETS_VERSION.to_string(),
        }
    }
}

/// Type for controller errors.
pub enum ControllerError {
    InternalError(String),
    NotFound(String),
}

/// Convert ServiceError into ControllerError.
/// They should generally be considered as internal errors unless explicitly handled.
impl From<ServiceError> for ControllerError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::InternalError(desc)
                => Self::InternalError(format!("Service error: {desc}")),
        }
    }
}

/// Convert ControllerError to error response.
/// TODO: have an actual error page.
impl IntoResponse for ControllerError {
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
        }.into_response()
    }
}
