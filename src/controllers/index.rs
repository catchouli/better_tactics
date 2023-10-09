use std::sync::Arc;

use askama::Template;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::route::{InternalError, BaseTemplateData};
use crate::srs;
use crate::time::LocalTimeProvider;
use crate::util;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplateData,
    user: User,
    stats: Stats,
}

pub async fn index_page(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<IndexTemplate, warp::Rejection>
{
    let user_id = PuzzleDatabase::local_user_id();
    let puzzle_db = puzzle_db.lock().await;

    // Get user details and stats.
    let user = puzzle_db.get_user_by_id(user_id).await
        .map_err(InternalError::from)?
        .ok_or(InternalError::new(format!("No local user record in database")))?;

    let review_cutoff = srs::day_end_datetime::<LocalTimeProvider>();
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).await.map_err(InternalError::from)?;

    Ok(IndexTemplate {
        base: Default::default(),
        user,
        stats,
    })
}
