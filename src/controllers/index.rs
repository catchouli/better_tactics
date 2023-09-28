use std::sync::Arc;

use askama::Template;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::PuzzleDatabase;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    card_count: i64,
    review_count: i64,
    reviews_due: i64,
    next_review_due: String,
}

pub async fn index_page(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<IndexTemplate, warp::Rejection>
{
    let puzzle_db = puzzle_db.lock().await;
    // TODO: unsafe unwrap
    let stats = puzzle_db.get_stats().unwrap();

    // Format 'next review due' time.
    let now = Local::now().fixed_offset();

    let next_review_due = if stats.next_review_due < now {
        "now".to_string()
    }
    else {
        let time_until_next_review = stats.next_review_due - now;

        let hours = time_until_next_review.num_hours();
        let mins = time_until_next_review.num_minutes() - hours * 60;
        let secs = time_until_next_review.num_seconds() - hours * 60 * 60 - mins * 60;

        format!("{}h {}m {}s", hours, mins, secs)
    };

    Ok(IndexTemplate {
        card_count: stats.card_count,
        review_count: stats.review_count,
        reviews_due: stats.reviews_due,
        next_review_due,
    })
}
