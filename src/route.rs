use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::Mutex;
use warp::reject::{self, Rejection};
use warp::{Filter, reply, reply::Reply, http::StatusCode};
use static_dir::static_dir;

use crate::controllers::index::IndexTemplate;
use crate::controllers::puzzle;
use crate::db::PuzzleDatabase;

/// Our error type for bad requests.
#[derive(Debug)]
pub struct InvalidParameter;
impl reject::Reject for InvalidParameter {}

/// Our routes.
pub fn routes(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> impl Filter::<Extract: Reply, Error = Infallible> + Clone + Send + Sync + 'static
{
    warp::path("assets").and(static_dir!("assets"))

        .or(warp::path::end()
            .map(move || IndexTemplate {}))

        .or(warp::path!("tactics" / "random")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                // A bit ugly and there's probably a better way to do this than cloning it twice...
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

        .or(warp::path!("tactics" / "review")
            .and(warp::path::end())
            .and(warp::get())
            .and_then({
                let puzzle_db = puzzle_db.clone();
                move || puzzle::random_puzzle(puzzle_db.clone())
            }))

        // TODO: figure out how to get post variables, and either redirect the user back to the
        // appropriate page, or use ajax.
        .or(warp::path!("review")
            .and(warp::path::end())
            .and(warp::post())
            .and_then({
                move || puzzle::review_puzzle(puzzle_db.clone(), "".to_string(), crate::srs::Difficulty::Good)
            }))

        .recover(handle_rejection)
}

/// Custom rejection handler that maps rejections into responses.
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    log::info!("Error handler");
    if err.is_not_found() {
        log::info!("Not found");
        Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
    }
    else if let Some(_) = err.find::<InvalidParameter>() {
        Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
    }
    else {
        Ok(reply::with_status("INTERNAL_SERVER_ERROR", StatusCode::INTERNAL_SERVER_ERROR))
    }
}
