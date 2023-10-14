use axum::Router;
use tower_http::services::ServeDir;

use crate::route::AppState;

pub mod index;
pub mod puzzle;
pub mod about;

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

        .with_state(app_state)
}
