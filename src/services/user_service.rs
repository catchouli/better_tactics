use std::sync::Arc;
use chrono::{DateTime, FixedOffset, Local};
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, ReviewScoreBucket};
use crate::rating::{Rating, GameResult};
use crate::srs::{self, Difficulty};
use crate::time::LocalTimeProvider;

use super::{ServiceResult, ServiceError};

/// Used for returning general user statistics.
#[derive(Debug, Clone)]
pub struct Stats {
    pub card_count: i64,
    pub review_count: i64,
    pub reviews_due_now: i64,
    pub reviews_due_today: i64,
    pub next_review_due: Option<DateTime<FixedOffset>>,
}

/// Encapsulates any kind of application logic to do with users.
#[derive(Clone)]
pub struct UserService {
    db: Arc<Mutex<PuzzleDatabase>>,
}

impl UserService {
    pub fn new(db: Arc<Mutex<PuzzleDatabase>>) -> Self {
        Self {
            db,
        }
    }

    /// Get the local user ID. (for now, we just have the local user, but if we ever want to turn
    /// this into a 'proper' web app, we can switch over to using an account system.)
    pub fn local_user_id() -> &'static str {
        "local"
    }

    /// Get the user's "next puzzle" if it's set.
    pub async fn get_user_next_puzzle(&self, user_id: &str) -> ServiceResult<Option<String>> {
        Ok(self.db.lock().await
           .get_user_by_id(user_id)
           .await?
           .map(|user| user.next_puzzle)
           .flatten())
    }

    /// Set the user's "next puzzle".
    pub async fn set_user_next_puzzle(&self, user_id: &str, next_puzzle: Option<&str>)
        -> ServiceResult<()>
    {
        let mut user = self.db.lock().await
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| ServiceError::InternalError(format!("No such user {user_id}")))?;

        user.next_puzzle = next_puzzle.map(ToString::to_string);
        
        self.db.lock().await
            .update_user(&user)
            .await?;

        Ok(())
    }

    /// Get the rating for a user.
    pub async fn get_user_rating(&self, user_id: &str) -> ServiceResult<Rating> {
        Ok(self.db.lock().await
            .get_user_by_id(user_id).await?
            .ok_or_else(|| format!("No such user with id {user_id}"))?
            .rating)
    }

    /// Reset a user's rating to the given value and rating deviation.
    pub async fn reset_user_rating(&self, user_id: &str, rating: i64, deviation: i64, volatility: f64)
        -> ServiceResult<()>
    {
        let mut db = self.db.lock().await;

        let mut user = db.get_user_by_id(user_id).await?
            .ok_or_else(|| format!("No such user with id {user_id}"))?;

        user.rating = Rating {
            rating,
            deviation,
            volatility
        };

        db.update_user(&user).await?;

        Ok(())
    }

    /// Get the stats for a user.
    pub async fn get_user_stats(&self, user_id: &str) -> ServiceResult<Stats> {
        Self::validate_user_id(user_id)?;

        let db = self.db.lock().await;

        // Get the current time and day end.
        let now = Local::now().fixed_offset();
        let day_end = srs::day_end_datetime::<LocalTimeProvider>();

        // Get the user's card count and review count.
        let card_count = db.get_card_count().await?;
        let review_count = db.get_review_count().await?;

        // Get the reviews due now and reviews due today.
        let reviews_due_now = db.reviews_due_by(now.clone(), day_end.clone()).await?;
        let reviews_due_today = db.reviews_due_by(day_end.clone(), day_end.clone()).await?;

        // Get when the next review is due if there aren't any due now.
        let next_review_due = match reviews_due_now {
            0 => db.get_next_review_due(day_end, None).await?.map(|(c, _)| c.due),
            _ => Some(now),
        };

        Ok(Stats {
            card_count,
            review_count,
            reviews_due_now,
            reviews_due_today,
            next_review_due,
        })
    }

    /// Get the review forecast for a user.
    pub async fn get_review_forecast(&self, user_id: &str, length_days: i64)
        -> ServiceResult<Vec<(i64, i64)>>
    {
        Self::validate_user_id(user_id)?;
        let db = self.db.lock().await;

        let day_end = srs::day_end_datetime::<LocalTimeProvider>();
        let review_forecast = db.get_review_forecast(day_end, length_days).await?;

        Ok(review_forecast)
    }

    /// Get the rating history for a user.
    pub async fn get_rating_history(&self, user_id: &str)
        -> ServiceResult<Vec<(DateTime<FixedOffset>, i64)>>
    {
        Ok(self.db.lock().await
            .get_user_rating_history(user_id)
            .await?)
    }

    /// Get the review score histogram for a user.
    pub async fn get_review_score_histogram(&self, user_id: &str, bucket_size: i64)
        -> ServiceResult<Vec<ReviewScoreBucket>>
    {
        Ok(self.db.lock().await
           .get_review_score_history(user_id, bucket_size)
           .await?)
    }

    /// Update the rating for a user.
    pub async fn update_rating(&self, user_id: &str, difficulty: Difficulty, result: GameResult<i64>)
        -> ServiceResult<Rating>
    {
        Self::validate_user_id(user_id)?;
        let mut db = self.db.lock().await;
        
        // Get the user.
        let mut user = db.get_user_by_id(user_id).await?
            .ok_or(ServiceError::from(format!("No such user {user_id}")))?;

        // Update the user's rating every time a puzzle is solved, old puzzles don't give much
        // rating anymore once your rating deviation is low enough.
        let old_rating = user.rating;
        user.rating.update(vec![result]);

        // The downside is that sometimes 'Good' ratings for low rated puzzles actually lower the
        // user's rating, due to the way we fudge the score for them. (They're scored somewhere
        // between 'Easy', which is a win at 1.0, and 'Hard', which is a draw at 0.0). As a bit of
        // an arbitrary fix, we just prevent 'Good' ratings from lowering the user's rating.
        if difficulty != Difficulty::Good || user.rating.rating > old_rating.rating {
            log::info!("Updating user's rating from {} to {}", old_rating.rating, user.rating.rating);
            db.update_user(&user).await?;

            Ok(user.rating)
        }
        else {
            Ok(old_rating)
        }
    }

    /// Validate a user id.
    fn validate_user_id(user_id: &str) -> ServiceResult<()> {
        // For now we only support a local user, so check that it's the local user's stats that are
        // being requested. In the future, we might also want to store the user's stats in the
        // users table and just update them as needed, to avoid having to look them up every time.
        if user_id == Self::local_user_id() {
            Ok(())
        }
        else {
            Err(format!("Invalid user id {user_id} in local user mode"))?
        }
    }
}
