use askama::Template;
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

/// The base template data.
pub struct BaseTemplateData {
    pub assets_version: String,
}

/// Internal server error template.
#[derive(Template, Default)]
#[template(path = "internal_server_error.html")]
pub struct InternalServerErrorTemplate {
    base: BaseTemplateData,
    error_details: String,
}

impl Default for BaseTemplateData {
    fn default() -> Self {
        Self {
            assets_version: ASSETS_VERSION.to_string(),
        }
    }
}

/// Not found template.
#[derive(Template, Default)]
#[template(path = "not_found.html")]
pub struct NotFoundTemplate {
    base: BaseTemplateData,
    request_uri: String,
}

/// Not found handler.
pub async fn not_found(req: Request<Body>) -> NotFoundTemplate {
    NotFoundTemplate {
        base: Default::default(),
        request_uri: req.uri().to_string(),
    }
}

/// Type for controller errors.
pub enum ControllerError {
    InternalError(String)
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
impl IntoResponse for ControllerError {
    fn into_response(self) -> askama_axum::Response {
        match self {
            Self::InternalError(desc) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                InternalServerErrorTemplate {
                    base: Default::default(),
                    error_details: desc,
                }
            ),
        }.into_response()
    }
}
