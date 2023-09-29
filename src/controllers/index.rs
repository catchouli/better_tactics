use std::sync::Arc;

use askama::Template;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::rating::GameResult;
use crate::route::InternalError;
use crate::srs::Difficulty;

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
    let stats = puzzle_db.get_user_stats(user_id)
        .map_err(InternalError::from)?;

    // Format 'next review due' time as a human readable time.
    let time_until_next_review = stats.next_review_due - Local::now().fixed_offset();
    let next_review_due_human = crate::util::review_duration_to_human(&time_until_next_review);

    // Calculate new user rating (temp).
    //let reviews = puzzle_db.last_n_reviews_for_user(user_id, 10)
    //    .map_err(InternalError::from)?;

    //for (review, rating) in &reviews {
    //    log::info!("Review (rating {rating}): {review:?}");
    //}

    //let game_results = reviews.into_iter().map(|(review, rating)| {
    //    GameResult {
    //        rating,
    //        deviation: 0,
    //        score: match review.difficulty {
    //            Difficulty::Again => 0.0,
    //            Difficulty::Hard => 0.5,
    //            Difficulty::Good => 1.0,
    //            Difficulty::Easy => 1.0,
    //        }
    //    }
    //}).collect();
    //user.rating.update(game_results);

    Ok(IndexTemplate {
        user,
        stats,
        next_review_due_human,
    })
}
