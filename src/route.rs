use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{Filter, reply, reply::Reply, http::StatusCode};
use warp::reject::{self, Rejection};

use crate::config::AppConfig;
use crate::controllers::{index, about, api};
use crate::controllers::puzzle::{self, ReviewRequest};
use crate::db::PuzzleDatabase;
use crate::services::ServiceError;
use crate::services::tactics_service::TacticsService;
use crate::services::user_service::UserService;

static ASSETS_VERSION: &'static str = env!("CARGO_PKG_VERSION");

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

/// Our routes.
pub fn routes(app_config: AppConfig, puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> impl Filter::<Extract = impl Reply> + Clone + Send + Sync + 'static
{
    // Instantiate services.
    let user_service = UserService::new(puzzle_db.clone());
    let tactics_service = TacticsService::new(puzzle_db.clone());

    // Serve the static assets.
    warp::path(format!("assets_{}", ASSETS_VERSION))
        .and(assets_filter())

        // GET / - the index page.
        .or(warp::path::end()
            .and_then({
                move || index::index_page()
            }))

        // Get /about - the about page.
        .or(warp::path!("about")
            .map(|| about::about_page()))

        // GET /tactics/new - shows a new, new tactics puzzle.
        .or(warp::path!("tactics" / "new")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                // A bit ugly and there's probably a better way to do this than cloning it twice...
                // TODO: just change away from warp to axum because this routing is kind of horrible.
                let user_service = user_service.clone();
                let tactics_service = tactics_service.clone();
                move || puzzle::random_puzzle(app_config.srs, user_service.clone(), tactics_service.clone())
            }))

        // GET /tactics/by_id/{id} - displays a tactics puzzle by id.
        .or(warp::path!("tactics" / "by_id" / String)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                let tactics_service = tactics_service.clone();
                move |id| puzzle::specific_puzzle(app_config.srs, user_service.clone(),
                    tactics_service.clone(), id)
            }))

        // GET /tactics - displays the user's next review.
        .or(warp::path!("tactics")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                let tactics_service = tactics_service.clone();
                move || puzzle::next_review(app_config.srs, user_service.clone(), tactics_service.clone())
            }))

        // POST /review - for submitting a review.
        .or(warp::path!("review")
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .and_then({
                let user_service = user_service.clone();
                let tactics_service = tactics_service.clone();
                move |request: ReviewRequest| {
                    log::info!("Got review request: {request:?}");
                    puzzle::review_puzzle(app_config.srs, user_service.clone(), tactics_service.clone(),
                        request)
                }
            }))

        // A temporary route that allows the user to set their rating without opening the sqlite
        // database externally. So if the new rating system causes ratings to go awry it's easy to
        // fix.
        .or(warp::path!("api" / "reset_rating" / i64)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                move |new_rating: i64| {
                    api::reset_user_rating(user_service.clone(), new_rating)
                }
            }))

        .or(warp::path!("api" / "user_stats")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                move || {
                    api::user_stats(user_service.clone())
                }
            }))

        .or(warp::path!("api" / "review_forecast" / i64)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                move |length_days: i64| {
                    api::review_forecast(user_service.clone(), length_days)
                }
            }))

        .or(warp::path!("api" / "rating_history")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                move || {
                    api::rating_history(user_service.clone())
                }
            }))

        .or(warp::path!("api" / "review_score_histogram" / i64)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let user_service = user_service.clone();
                move |bucket_size: i64| {
                    api::review_score_histogram(user_service.clone(), bucket_size)
                }
            }))

        .recover(handle_rejection)
}

/// Our assets filter. In debug, we just want to serve the directory directly, but in release we
/// actually want to build the assets into the exe so it can be distributed.
/// Using the cfg(debug_assertions) seems weird for this, but apparently it is the right way to go:
/// https://stackoverflow.com/questions/39204908/how-to-check-release-debug-builds-using-cfg-in-rust
#[cfg(debug_assertions)]
fn assets_filter() -> impl Filter<Extract = (warp::fs::File,), Error = Rejection> + Clone {
    warp::fs::dir("./assets")
}

/// The assets filter for release mode, which uses the static_dir crate to embed the assets in our
/// build.
#[cfg(not(debug_assertions))]
fn assets_filter() -> impl Filter<Extract = (warp::reply::Response,), Error = Rejection> + Clone {
    use static_dir::static_dir;
    static_dir!("assets")
}

/// Implement reject::Reject for custom error types.
impl reject::Reject for ServiceError {}

/// Custom rejection handler that maps rejections into responses.
/// TODO: return error page instead of message only.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let (reply, status) = if err.is_not_found() {
        let reply = warp::reply::html(format!("Page not found"));
        (reply, StatusCode::NOT_FOUND)
    }
    else if let Some(err) = err.find::<ServiceError>() {
        match err {
            ServiceError::InternalError(desc) => (
                warp::reply::html(format!("Internal error: {desc}")),
                StatusCode::INTERNAL_SERVER_ERROR
            ),
            ServiceError::InvalidParameter(param) => (
                warp::reply::html(format!("Bad request: parameter {param}")),
                StatusCode::BAD_REQUEST
            )
        }
    }
    else {
        log::error!("Internal error of unknown type! {err:?}");
        let reply = warp::reply::html(format!("Internal error"));
        (reply, StatusCode::INTERNAL_SERVER_ERROR)
    };

    Ok(reply::with_status(reply, status))
}

