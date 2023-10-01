use std::sync::Arc;

use askama::Template;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::route::InternalError;
use crate::srs::Card;

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
    let user_id = PuzzleDatabase::local_user_id();
    let puzzle_db = puzzle_db.lock().await;

    // Get user details and stats.
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .unwrap();

    let review_cutoff = Card::due_time().map_err(InternalError::from)?;
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).map_err(InternalError::from)?;

    // Format 'next review due' time as a human readable time.
    let time_until_next_review = stats.next_review_due - Local::now().fixed_offset();
    let next_review_due_human = crate::util::review_duration_to_human(&time_until_next_review);

    Ok(IndexTemplate {
        user,
        stats,
        next_review_due_human,
    })
}
