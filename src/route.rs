use std::fmt::Display;
use std::sync::Arc;
use std::error::Error;

use tokio::sync::Mutex;
use warp::{Filter, reply, reply::Reply, http::StatusCode};
use warp::reject::{self, Rejection};

use crate::controllers::index;
use crate::controllers::puzzle::{self, ReviewRequest};
use crate::db::PuzzleDatabase;

/// Our error type for requests when a specified resource was not found.
#[derive(Debug)]
pub struct NotFound {
    pub resource: String,
}

impl NotFound {
    pub fn new(resource: &str) -> Self {
        Self {
            resource: resource.to_string()
        }
    }
}

impl Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The requested resource was not found: {}", self.resource)
    }
}

impl reject::Reject for NotFound {}

/// Our error type for bad requests.
#[derive(Debug)]
pub struct InvalidParameter {
    pub param: String,
}

impl InvalidParameter {
    pub fn new(param: &str) -> Self {
        Self {
            param: param.to_string()
        }
    }
}

impl Display for InvalidParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid parameter {}", self.param)
    }
}

impl reject::Reject for InvalidParameter {}

/// Our error type for internal errors.
#[derive(Debug)]
pub struct InternalError {
    pub description: String,
}

impl InternalError {
    pub fn new(description: String) -> Self {
        Self {
            description,
        }
    }

    /// Get the appropriate message to return to the user for an internal error. When debug assertions
    /// are enabled, we return the original message.
    #[cfg(debug_assertions)]
    fn user_message(&self) -> String {
        format!("Internal error: {}", self.description)
    }

    /// Get the appropriate message to return to the user for an internal error. When debug assertions
    /// are not enabled, we simply return that there was an internal server error, to avoid leaking
    /// any internals.
    #[cfg(not(debug_assertions))]
    fn user_message(&self) -> String {
        format!("Internal server error")
    }
}

impl From<Box<dyn Error>> for InternalError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value.to_string())
    }
}

impl reject::Reject for InternalError {}

/// Our routes.
pub fn routes(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> impl Filter::<Extract: Reply> + Clone + Send + Sync + 'static
{
    // Serve the static assets.
    warp::path("assets").and(assets_filter())

        // GET / - the index page.
        .or(warp::path::end()
            .and_then({
                // A bit ugly and there's probably a better way to do this than cloning it twice...
                let puzzle_db = puzzle_db.clone();
                move || index::index_page(puzzle_db.clone())
            }))

        // GET /tactics/new - shows a new, new tactics puzzle.
        .or(warp::path!("tactics" / "new")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move || puzzle::random_puzzle(puzzle_db.clone())
            }))

        // GET /tactics/single/{id} - displays a tactics puzzle by id.
        .or(warp::path!("tactics" / String)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move |id| puzzle::specific_puzzle(puzzle_db.clone(), id)
            }))

        // GET /tactics - displays the user's next review.
        .or(warp::path!("tactics")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move || puzzle::next_review(puzzle_db.clone())
            }))

        // POST /review - for submitting a review.
        .or(warp::path!("review")
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move |request: ReviewRequest| {
                    log::info!("Got review request: {request:?}");
                    puzzle::review_puzzle(puzzle_db.clone(), request)
                }
            }))

        // A temporary route that allows the user to set their rating without opening the sqlite
        // database externally. So if the new rating system causes ratings to go awry it's easy to
        // fix.
        .or(warp::path!("set_rating" / i64)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move |new_rating: i64| {
                    set_rating(puzzle_db.clone(), new_rating)
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

async fn set_rating(puzzle_db: Arc<Mutex<PuzzleDatabase>>, new_rating: i64)
    -> Result<impl warp::Reply, warp::Rejection>
{
    log::info!("Manually resetting user's rating to {new_rating}");

    let mut puzzle_db = puzzle_db.lock().await;
    let user_id = PuzzleDatabase::local_user_id();

    let mut user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

    user.rating.rating = new_rating;
    user.rating.deviation = 250;

    puzzle_db.update_user(&user)
        .map(|_| Ok(warp::reply::html(format!("Reset user rating to {new_rating}"))))
        .map_err(InternalError::from)?
}

/// Custom rejection handler that maps rejections into responses.
/// TODO: return error page instead of message only.
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let (reply, status) = if err.is_not_found() {
        let reply = warp::reply::html(format!("Page not found"));
        (reply, StatusCode::NOT_FOUND)
    }
    else if let Some(err) = err.find::<NotFound>() {
        let reply = warp::reply::html(format!("{}", err));
        (reply, StatusCode::NOT_FOUND)
    }
    else if let Some(err) = err.find::<InvalidParameter>() {
        let reply = warp::reply::html(format!("Bad request: {}", err));
        (reply, StatusCode::NOT_FOUND)
    }
    else if let Some(err) = err.find::<InternalError>() {
        log::error!("Internal error: {}", err.description);
        let reply = warp::reply::html(format!("{}", err.user_message()));
        (reply, StatusCode::INTERNAL_SERVER_ERROR)
    }
    else {
        log::error!("Internal error of unknown type! {err:?}");
        let reply = warp::reply::html(format!("Internal error"));
        (reply, StatusCode::INTERNAL_SERVER_ERROR)
    };

    Ok(reply::with_status(reply, status))
}

