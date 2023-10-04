use std::sync::Arc;

use askama::Template;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::route::InternalError;
use crate::srs::Card;
use crate::util;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    user: User,
    stats: Stats,
}

pub async fn index_page(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<IndexTemplate, warp::Rejection>
{
    let user_id = PuzzleDatabase::local_user_id();
    let puzzle_db = puzzle_db.lock().await;

    // Get user details and stats.
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .unwrap();

    let review_cutoff = Card::due_time().map_err(InternalError::from)?;
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).map_err(InternalError::from)?;

    Ok(IndexTemplate {
        user,
        stats,
    })
}
