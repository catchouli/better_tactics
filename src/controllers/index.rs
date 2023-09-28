use std::sync::Arc;

use askama::Template;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::route::InternalError;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    user: User,
    stats: Stats,
    next_review_due_human: String,
}

pub async fn index_page(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<IndexTemplate, warp::Rejection>
{
    let puzzle_db = puzzle_db.lock().await;

    // Get user details and stats.
    // TODO: unsafe unwraps.
    // TODO: the warp reject returns 404 by default, we need to handle all the errors more
    // appropriately or it's going to get confusing when they happen.
    let user = puzzle_db.get_user_by_id(PuzzleDatabase::local_user_id())
        .map_err(InternalError::from)?
        .unwrap();
    let stats = puzzle_db.get_local_user_stats()
        .map_err(InternalError::from)?;

    // Format 'next review due' time as a human readable time.
    let now = Local::now().fixed_offset();
    let time_until_next_review = stats.next_review_due - now;
    let next_review_due_human = crate::util::review_duration_to_human(&time_until_next_review);

    Ok(IndexTemplate {
        user,
        stats,
        next_review_due_human,
    })
}
