use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::config::AppConfig;
use crate::db::PuzzleDatabase;
use crate::services::ServiceError;
use crate::services::tactics_service::TacticsService;
use crate::services::user_service::UserService;

const ASSETS_VERSION: &str = env!("CARGO_PKG_VERSION");
const ASSETS_PATH: &str = concat!("/assets_", env!("CARGO_PKG_VERSION"));

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

/// The app state.
#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub user_service: UserService,
    pub tactics_service: TacticsService,
}

impl AppState {
    pub fn new(app_config: AppConfig, db: Arc<Mutex<PuzzleDatabase>>) -> AppState {
        Self {
            app_config,
            user_service: UserService::new(db.clone()),
            tactics_service: TacticsService::new(db.clone())
        }
    }
}

/// Type for controller errors.
pub enum ControllerError {
    InternalError(String),
    InvalidParameter(String),
    NotFound(String),
}

/// Convert ServiceError into ControllerError.
/// They should generally be considered as internal errors unless explicitly handled.
impl From<ServiceError> for ControllerError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::InternalError(desc)
                => Self::InternalError(format!("Service error: {desc}")),
            ServiceError::InvalidParameter(param)
                => Self::InternalError(format!("Invalid service parameter: {param}")),
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
            Self::InvalidParameter(param) => (
                StatusCode::BAD_REQUEST,
                format!("Bad request: invalid parameter {param}"),
            ),
        }.into_response()
    }
}
