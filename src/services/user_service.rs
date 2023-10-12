use std::sync::Arc;
use chrono::{DateTime, FixedOffset, Local, Duration, NaiveDate};
use tokio::sync::Mutex;

use crate::db::PuzzleDatabase;
use crate::rating::{Rating, GameResult};
use crate::srs::{self, Difficulty};
use crate::time::LocalTimeProvider;

use super::{ServiceResult, ServiceError};

/// The maximum number of days to show in the review forecast.
const REVIEW_FORECAST_LENGTH: usize = 8;

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
    pub async fn get_review_forecast(&self, user_id: &str) -> ServiceResult<Vec<i64>> {
        Self::validate_user_id(user_id)?;
        let db = self.db.lock().await;

        let mut review_forecast = Vec::new();

        // Get the reviews due today as the initial value.
        let day_end = srs::day_end_datetime::<LocalTimeProvider>();
        review_forecast.push(db.reviews_due_by(day_end.clone(), day_end.clone()).await?);

        // Start at the end of today and look up the reviews due for each day in the following week
        // (or REVIEW_FORECAST_LENGTH days).
        let mut start = day_end;
        for _ in 0..REVIEW_FORECAST_LENGTH {
            let end = start + Duration::days(1);
            review_forecast.push(db.reviews_due_between(start, end).await?);
            start = end;
        }

        Ok(review_forecast)
    }

    /// Get the rating history for a user.
    pub async fn get_rating_history(&self, user_id: &str)
        -> ServiceResult<Vec<(NaiveDate, i64)>>
    {
        Self::validate_user_id(user_id)?;

        // Placeholder data.
        let res = vec![
            (NaiveDate::from_ymd_opt(2023, 08, 01).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 02).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 03).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 04).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 05).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 06).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 07).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 08).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 09).unwrap(), 1000),
            (NaiveDate::from_ymd_opt(2023, 08, 10).unwrap(), 1100),
            (NaiveDate::from_ymd_opt(2023, 08, 11).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 12).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 13).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 14).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 15).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 16).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 17).unwrap(), 1150),
            (NaiveDate::from_ymd_opt(2023, 08, 18).unwrap(), 1100),
            (NaiveDate::from_ymd_opt(2023, 08, 19).unwrap(), 1100),
            (NaiveDate::from_ymd_opt(2023, 08, 20).unwrap(), 1100),
            (NaiveDate::from_ymd_opt(2023, 08, 21).unwrap(), 1200),
            (NaiveDate::from_ymd_opt(2023, 08, 22).unwrap(), 1200),
            (NaiveDate::from_ymd_opt(2023, 08, 23).unwrap(), 1200),
            (NaiveDate::from_ymd_opt(2023, 08, 24).unwrap(), 1200),
            (NaiveDate::from_ymd_opt(2023, 08, 25).unwrap(), 1200),
            (NaiveDate::from_ymd_opt(2023, 08, 26).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 08, 27).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 08, 28).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 08, 29).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 08, 30).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 01).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 02).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 03).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 04).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 05).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 06).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 07).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 08).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 09).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 10).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 11).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 12).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 13).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 14).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 15).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 16).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 17).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 18).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 19).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 20).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 21).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 22).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 23).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 24).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 25).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 26).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 27).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 28).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 29).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 09, 30).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 10, 07).unwrap(), 1400),
            (NaiveDate::from_ymd_opt(2023, 10, 08).unwrap(), 1500),
            (NaiveDate::from_ymd_opt(2023, 10, 09).unwrap(), 1600),
            (NaiveDate::from_ymd_opt(2023, 10, 10).unwrap(), 1700),
        ];

        Ok(res)
    }

    /// Update the rating for a user.
    pub async fn update_rating(&self, user_id: &str, difficulty: Difficulty, result: GameResult<i64>)
        -> ServiceResult<()>
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
        }

        Ok(())
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
