use std::convert::Infallible;
use std::sync::Arc;
use std::error::Error;

use tokio::sync::Mutex;
use warp::reject::{self, Rejection};
use warp::{Filter, reply, reply::Reply, http::StatusCode};
use static_dir::static_dir;

use crate::controllers::index;
use crate::controllers::puzzle::{self, ReviewRequest};
use crate::db::PuzzleDatabase;

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
}

impl From<Box<dyn Error>> for InternalError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value.to_string())
    }
}

impl reject::Reject for InternalError {}

/// Our routes.
pub fn routes(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> impl Filter::<Extract: Reply, Error = Infallible> + Clone + Send + Sync + 'static
{
    warp::path("assets").and(static_dir!("assets"))

        .or(warp::path::end()
            .and_then({
                // A bit ugly and there's probably a better way to do this than cloning it twice...
                let puzzle_db = puzzle_db.clone();
                move || index::index_page(puzzle_db.clone())
            }))

        .or(warp::path!("tactics" / "random")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move || puzzle::random_puzzle(puzzle_db.clone())
            }))

        .or(warp::path!("tactics" / "single" / String)
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move |id| puzzle::specific_puzzle(puzzle_db.clone(), id)
            }))

        .or(warp::path!("tactics")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move || puzzle::next_review(puzzle_db.clone())
            }))

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
async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    log::info!("Error handler");
    if err.is_not_found() {
        log::info!("Not found");
        Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
    }
    else if let Some(err) = err.find::<InvalidParameter>() {
        log::warn!("Invalid parameter: {}", err.param);
        Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
    }
    else if let Some(err) = err.find::<InternalError>() {
        log::error!("Internal error: {}", err.description);
        Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
    }
    else {
        log::error!("Unspecified internal error!");
        Ok(reply::with_status("INTERNAL_SERVER_ERROR", StatusCode::INTERNAL_SERVER_ERROR))
    }
}
