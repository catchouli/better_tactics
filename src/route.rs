use std::convert::Infallible;
use std::sync::Arc;

use tokio::sync::Mutex;
use warp::reject::{self, Rejection};
use warp::{Filter, reply, reply::Reply, http::StatusCode};

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
    warp::path("assets").and(warp::fs::dir("./assets"))
        .or(warp::path::end().map(move || IndexTemplate {}))
        .or(warp::path("tactics").and_then(move || puzzle::random_puzzle(puzzle_db.clone()) ))
        .recover(handle_rejection)
}

/// Custom rejection handler that maps rejections into responses.
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    if err.is_not_found() {
        Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
    }
    else if let Some(_) = err.find::<InvalidParameter>() {
        Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
    }
    else {
        Ok(reply::with_status("INTERNAL_SERVER_ERROR", StatusCode::INTERNAL_SERVER_ERROR))
    }
}
