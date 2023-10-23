use chrono::Local;

use crate::app::AppConfig;
use crate::db::{PuzzleDatabase, Puzzle, Review, PuzzleHistoryEntry, UserId, PuzzleId};
use crate::rating::Rating;
use crate::srs::{Card, Difficulty, CardId};
use crate::time::LocalTimeProvider;

use super::{ServiceResult, ServiceError};

/// Encapsulates any kind of application logic to do with tactics.
#[derive(Clone)]
pub struct TacticsService {
    pub app_config: AppConfig,
    pub db: PuzzleDatabase,
}

impl TacticsService {
    pub fn new(app_config: AppConfig, db: PuzzleDatabase) -> Self {
        Self {
            app_config,
            db,
        }
    }

    pub async fn get_puzzle_and_card_by_puzzle_id(&self, user_id: UserId, puzzle_id: PuzzleId)
        -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let puzzle = self.db.get_puzzle_by_id(puzzle_id).await?;
        let card = self.db.get_card_for_puzzle(user_id, puzzle_id).await?;

        Ok((puzzle, card))
    }

    pub async fn get_puzzle_and_card_by_source_id(&self, user_id: UserId, source_name: &str,
        source_id: &str) -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        let puzzle = self.db.get_puzzle_by_source_id(source_name, source_id).await?;

        let card = match puzzle.as_ref() {
            Some(puzzle) => {
                let puzzle_id = puzzle.id
                    .ok_or_else(|| ServiceError::InternalError("Got puzzle with no id".into()))?;
                self.db.get_card_for_puzzle(user_id, puzzle_id).await?
            },
            _ => None
        };

        Ok((puzzle, card))
    }

    pub async fn get_puzzle_and_card_by_card_id(&self, user_id: UserId, card_id: CardId)
        -> ServiceResult<Option<(Puzzle, Card)>>
    {
        if let Some(card) = self.db.get_card_by_id(user_id, card_id).await? {
            if let Some(puzzle) = self.db.get_puzzle_by_id(card.puzzle_id).await? {
                Ok(Some((puzzle, card)))
            }
            else {
                Ok(None)
            }
        }
        else {
            Ok(None)
        }
    }

    pub async fn get_next_review(&self) -> ServiceResult<Option<(Puzzle, Card)>>
    {
        // Get the user's next due review. The logic for this is a little complicated as we want to
        // show cards that are due any time today (before the review cutoff) so the user can do their
        // reviews all at once rather than them trickling in throughout the day. On the other hand, we
        // don't want cards that are still in learning, with potentially low intervals, to show up
        // before they're due unless there's absolutely no other cards left, because otherwise they'll
        // show up repeatedly in front of other cards that are due later today.
        let time_now = Local::now().fixed_offset();
        let next_review_due_now = self.db.get_next_review_due(time_now, None).await?;

        let max_learning_interval = crate::srs::INITIAL_INTERVALS.last().map(|d| *d);
        let review_cutoff_today = self.app_config.srs.day_end_datetime::<LocalTimeProvider>();
        let next_non_learning_review_due_today =
            self.db.get_next_review_due(review_cutoff_today, max_learning_interval).await?;

        if let Some(card) = next_review_due_now.or(next_non_learning_review_due_today) {
            let puzzle = self.db.get_puzzle_by_id(card.puzzle_id).await?
                .ok_or_else(|| format!("Failed to get puzzle for card {card:?}"))?;
            Ok(Some((puzzle, card)))
        }
        else {
            Ok(None)
        }
    }

    pub async fn get_random_puzzle(&self, user_id: UserId, min_rating: i64, max_rating: i64)
        -> ServiceResult<(Option<Puzzle>, Option<Card>)>
    {
        // Clamp min and max rating to those of the puzzle database, or the request may come back
        // with nothing.
        let min_puzzle_rating = self.db.get_min_puzzle_rating().await?;
        let max_puzzle_rating = self.db.get_max_puzzle_rating().await?;
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

            puzzle = self.db.get_puzzles_by_rating(min_rating, max_rating, 1).await?
                .into_iter().next();

            if let Some(puzzle) = puzzle.as_ref() {
                let puzzle_id = puzzle.id
                    .ok_or_else(|| ServiceError::InternalError("Got puzzle with no id".into()))?;
                card = self.db.get_card_for_puzzle(user_id, puzzle_id).await?;
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

    pub async fn apply_review(&mut self, user_id: UserId, puzzle_id: PuzzleId, user_rating: Rating,
                              mut card: Card, difficulty: Difficulty) -> ServiceResult<()>
    {
        // Apply the review to the card.
        log::info!("Reviewing card");
        card.review(Local::now().fixed_offset(), difficulty);

        // Update (or create) the card in the database.
        log::info!("Updating card");
        self.db.update_or_create_card(&card).await?;

        // Create a review record in the database.
        log::info!("Adding review for user");
        self.db.add_review_for_user(Review {
            user_id,
            puzzle_id,
            difficulty,
            date: Local::now().fixed_offset(),
            user_rating: Some(user_rating.rating),
        }).await?;

        Ok(())
    }

    pub async fn get_puzzle_history(&self, user_id: UserId, offset: i64, count: i64)
        -> ServiceResult<(Vec<PuzzleHistoryEntry>, i64)>
    {
        Ok(self.db
            .get_distinct_puzzle_history(user_id, offset, count)
            .await?)
    }

    pub async fn skip_puzzle(&mut self, user_id: UserId, puzzle: &Puzzle)
        -> ServiceResult<()>
    {
        let puzzle_id = puzzle.id
            .ok_or_else(|| ServiceError::InternalError("Got puzzle with no id".into()))?;
        if self.db.get_puzzle_by_id(puzzle_id).await?.is_some() {
            let time_now = Local::now().fixed_offset();
            self.db.add_skipped_puzzle(user_id, puzzle_id, time_now).await?;
        }

        Ok(())
    }
}
