use chrono::{DateTime, FixedOffset, Local};
use sqlx::sqlite::*;
use sqlx::Row;

use crate::rating::Rating;
use crate::db::{PuzzleDatabase, DbResult, DatabaseError, ErrorDetails};

/// A stats record from the db (for the local user, for now).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Stats {
    pub card_count: i64,
    pub review_count: i64,
    pub reviews_due_now: i64,
    pub reviews_due_today: i64,
    pub next_review_due: Option<DateTime<FixedOffset>>,
}

/// A user record from the db.
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub rating: Rating,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for User
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            rating: Rating {
                rating: row.try_get("rating")?,
                deviation: row.try_get("rating_deviation")?,
                volatility: row.try_get("rating_volatility")?,
            },
        })
    }
}

impl PuzzleDatabase {
    /// Get the stats for the local user.
    pub async fn get_user_stats(&self, user_id: &str, day_end: DateTime<FixedOffset>) -> DbResult<Stats> {
        // For now we only support a local user, so check that it's the local user's stats that are
        // being requested. In the future, we might also want to store the user's stats in the
        // users table and just update them as needed, to avoid having to look them up every time.
        if user_id != Self::local_user_id() {
            return Err(DatabaseError::ParameterError(ErrorDetails {
                backend: "".to_string(),
                source: None,
                description: "user_id".to_string(),
            }));
        }

        let now = Local::now().fixed_offset();
        let reviews_due_now = self.reviews_due_by(now.clone(), day_end.clone()).await?;
        let next_review_due = if reviews_due_now > 0 {
            Some(now)
        } else {
            self.get_next_review_due(day_end, None).await?.map(|(c, _)| c.due)
        };

        Ok(Stats {
            card_count: self.get_card_count().await?,
            review_count: self.get_review_count().await?,
            reviews_due_now,
            reviews_due_today: self.reviews_due_by(day_end.clone(), day_end.clone()).await?,
            next_review_due,
        })
    }

    /// Get the local user ID. (for now, we just have the local user, but if we ever want to turn
    /// this into a 'proper' web app, we can switch over to using an account system.)
    pub fn local_user_id() -> &'static str {
        "local"
    }

    /// Get the user record with the given ID.
    pub async fn get_user_by_id(&self, user_id: &str) -> DbResult<Option<User>> {
        sqlx::query_as("
            SELECT *
            FROM users
            WHERE id = ?
        ")
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await.map_err(Into::into)
    }

    /// Update the user record with the given ID.
    pub async fn update_user(&mut self, user: &User) -> DbResult<()> {
        sqlx::query("
            UPDATE users
            SET rating = ?,
                rating_deviation = ?,
                rating_volatility = ?
            WHERE id = ?
        ")
        .bind(user.rating.rating)
        .bind(user.rating.deviation)
        .bind(user.rating.volatility)
        .bind(&user.id)
        .execute(&self.pool)
        .await.map(|_| ()).map_err(Into::into)
    }
}
