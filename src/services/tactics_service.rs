use std::sync::Arc;
use chrono::Local;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Puzzle, Review, PuzzleHistoryEntry};
use crate::rating::Rating;
use crate::srs::{Card, Difficulty, self};
use crate::time::LocalTimeProvider;

use super::ServiceResult;

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

    pub async fn get_next_review(&self) -> ServiceResult<Option<(Puzzle, Card)>>
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

        Ok(next_review_due_now
            .or(next_non_learning_review_due_today)
            .map(|(a, b)| (b, a)))
    }

    pub async fn get_random_puzzle(&self, min_rating: i64, max_rating: i64)
        -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let db = self.db.lock().await;

        // Clamp min and max rating to those of the puzzle database, or the request may come back
        // with nothing.
        let min_puzzle_rating = db.get_min_puzzle_rating().await?;
        let max_puzzle_rating = db.get_max_puzzle_rating().await?;
        let min_rating = i64::clamp(min_rating, min_puzzle_rating, max_puzzle_rating);
        let max_rating = i64::clamp(max_rating, min_puzzle_rating, max_puzzle_rating);

        // The puzzles database is pretty big and we still use string ids to refer to the cards
        // so it's quite slow in sqlite to join them to find out if there's already a card
        // associated with the puzzle. Instead, we just retry a few times to make it unlikely to
        // ever happen. If it does return a puzzle the user already has a card for, that just means
        // they're reviewing ahead, and isn't the end of the world.
        const NEW_RETRY_COUNT: usize = 5;

        let mut puzzle = None;
        let mut card = None;

        for retry in 0..NEW_RETRY_COUNT {
            if retry > 0 {
                log::warn!("Retry {retry} of trying to get a new random puzzle");
            }

            puzzle = db.get_puzzles_by_rating(min_rating, max_rating, 1).await?
                .into_iter().next();

            if let Some(puzzle) = puzzle.as_ref() {
                card = db.get_card_by_id(&puzzle.puzzle_id).await?;
                // If we got a new puzzle that doesn't already have a card associated, we can just
                // return at this point. Otherwise we try again up to NEW_RETRY_COUNT times.
                if card.is_none() {
                    break;
                }
            }
            else {
                // If we didn't get a puzzle at all we can just break since we aren't going to get one
                // next time.
                break;
            }
        }

        Ok((puzzle, card))
    }

    pub async fn apply_review(&self, user_id: &str, user_rating: Rating, mut card: Card,
        difficulty: Difficulty) -> ServiceResult<()>
    {
        // Apply the review to the card.
        log::info!("Reviewing card");
        card.review(Local::now().fixed_offset(), difficulty);

        // Update (or create) the card in the database.
        let mut db = self.db.lock().await;
        log::info!("Updating card");
        db.update_or_create_card(&card).await?;

        // Create a review record in the database.
        log::info!("Adding review for user");
        db.add_review_for_user(Review {
            user_id: user_id.to_string(),
            puzzle_id: card.id.to_string(),
            difficulty,
            date: Local::now().fixed_offset(),
            user_rating: Some(user_rating.rating),
        }).await?;

        Ok(())
    }

    pub async fn get_puzzle_history(&self, user_id: &str, offset: i64, count: i64)
        -> ServiceResult<(Vec<PuzzleHistoryEntry>, i64)>
    {
        Ok(self.db.lock()
            .await
            .get_distinct_puzzle_history(user_id, offset, count)
            .await?)
    }

    pub async fn skip_puzzle(&self, user_id: &str, puzzle: &Puzzle)
        -> ServiceResult<()>
    {
        let mut db = self.db.lock().await;

        if db.get_puzzle_by_id(&puzzle.puzzle_id).await?.is_some() {
            let time_now = Local::now().fixed_offset();
            db.add_skipped_puzzle(user_id, &puzzle.puzzle_id, time_now).await?;
        }

        Ok(())
    }
}
