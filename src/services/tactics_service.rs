use std::sync::Arc;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Puzzle, Review};
use crate::rating::Rating;
use crate::srs::{Card, Difficulty, self};
use crate::time::LocalTimeProvider;

use super::ServiceResult;

/// The rating variation for puzzles, in percent.
const PUZZLE_RATING_VARIATION: f64 = 1.05;

/// Encapsulates any kind of application logic to do with tactics.
#[derive(Clone)]
pub struct TacticsService {
    pub db: Arc<Mutex<PuzzleDatabase>>,
}

impl TacticsService {
    pub fn new(db: Arc<Mutex<PuzzleDatabase>>) -> Self {
        Self {
            db,
        }
    }

    /// Returns the min and max rating for puzzles, deviating from the user's rating by up to
    /// `PUZZLE_RATING_VARIATION`.
    pub async fn get_rating_range(&self, user_rating: &Rating) -> ServiceResult<(i64, i64)> {
        // Clamp the user's rating to the highest puzzle rating just in case there aren't any at
        // the user's rating level.
        let max_puzzle_rating = self.db.lock().await
            .get_max_puzzle_rating().await?;
        let base_rating = i64::min(user_rating.rating, max_puzzle_rating);

        let min_rating = (base_rating as f64 / PUZZLE_RATING_VARIATION) as i64;
        let max_rating = (base_rating as f64 * PUZZLE_RATING_VARIATION) as i64;

        Ok((min_rating, max_rating))
    }

    pub async fn get_puzzle_by_id(&self, puzzle_id: &str)
        -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let db = self.db.lock().await;

        let puzzle = db.get_puzzle_by_id(puzzle_id).await?;

        let card = match puzzle.as_ref() {
            Some(puzzle) => db.get_card_by_id(&puzzle.puzzle_id).await?,
            _ => None
        };

        Ok((puzzle, card))
    }

    pub async fn get_next_review(&self) -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let db = self.db.lock().await;

        // Get the user's next due review. The logic for this is a little complicated as we want to
        // show cards that are due any time today (before the review cutoff) so the user can do their
        // reviews all at once rather than them trickling in throughout the day. On the other hand, we
        // don't want cards that are still in learning, with potentially low intervals, to show up
        // before they're due unless there's absolutely no other cards left, because otherwise they'll
        // show up repeatedly in front of other cards that are due later today.
        let time_now = Local::now().fixed_offset();
        let next_review_due_now = db.get_next_review_due(time_now, None).await?;

        let max_learning_interval = crate::srs::INITIAL_INTERVALS.last().map(|d| *d);
        let review_cutoff_today = srs::day_end_datetime::<LocalTimeProvider>();
        let next_non_learning_review_due_today =
            db.get_next_review_due(review_cutoff_today, max_learning_interval).await?;

        let next_review_due = next_review_due_now.or(next_non_learning_review_due_today);

        match next_review_due {
            Some((card, puzzle)) => Ok((Some(puzzle), Some(card))),
            _ => Ok((None, None))
        }
    }

    pub async fn get_random_puzzle(&self, min_rating: i64, max_rating: i64)
        -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let db = self.db.lock().await;

        let puzzle = db.get_puzzles_by_rating(min_rating, max_rating, 1).await?
            .into_iter().next();

        let card = match puzzle.as_ref() {
            Some(puzzle) => db.get_card_by_id(&puzzle.puzzle_id).await?,
            _ => None
        };

        Ok((puzzle, card))
    }

    pub async fn apply_review(&self, user_id: &str, user_rating: Rating, mut card: Card,
        difficulty: Difficulty) -> ServiceResult<()>
    {
        // Apply the review to the card.
        card.review(Local::now().fixed_offset(), difficulty);

        // Update (or create) the card in the database.
        let mut db = self.db.lock().await;
        db.update_or_create_card(&card).await?;

        // Create a review record in the database.
        db.add_review_for_user(Review {
            user_id: user_id.to_string(),
            puzzle_id: card.id.to_string(),
            difficulty,
            date: Local::now().fixed_offset(),
            user_rating: Some(user_rating.rating),
        }).await?;

        Ok(())
    }
}
