use chrono::{DateTime, FixedOffset, Local};

use crate::rating::Rating;
use crate::db::{PuzzleDatabase, DbResult, DatabaseError, ErrorDetails};

/// A stats record from the db (for the local user, for now).
#[derive(Debug, Clone)]
pub struct Stats {
    pub card_count: i64,
    pub review_count: i64,
    pub reviews_due: i64,
    pub next_review_due: DateTime<FixedOffset>,
}

/// A user record from the db.
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub rating: Rating,
}

impl PuzzleDatabase {
    /// Get the stats for the local user.
    pub fn get_user_stats(&self, user_id: &str, card_due_time: DateTime<FixedOffset>) -> DbResult<Stats> {
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

        // Get card and review count.
        const QUERY: &'static str = "
            SELECT
                COUNT(*) AS card_count,
                COALESCE(SUM(review_count), 0) AS review_count
            FROM cards
        ";

        let (card_count, review_count) = self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                Ok((
                    Self::try_read::<i64>(&row, "card_count")?,
                    Self::try_read::<i64>(&row, "review_count")?,
                )) as DbResult<(i64, i64)>
            })
            .unwrap_or(Ok((0, 0)))?;

        // Get the number of reviews due.
        const QUERY_2: &'static str = "
            SELECT COUNT(*) as reviews_due
            FROM cards
            WHERE datetime(due) < datetime(?)
        ";

        let reviews_due = self.conn
            .prepare(QUERY_2).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, card_due_time.to_rfc3339().as_str())).map_err(Self::convert_error)?
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Ok(Self::try_read::<i64>(&row, "reviews_due")?) as DbResult<i64>
            })
            .unwrap_or(Ok(0))?;

        // Get next review due time.
        const QUERY_3: &'static str = "
            SELECT due
            FROM cards
            ORDER BY datetime(due) ASC
            LIMIT 1
        ";

        let next_review_due = self.conn
            .prepare(QUERY_3).map_err(Self::convert_error)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                let due = Self::try_parse_datetime(Self::try_read(&row, "due")?)?;
                Ok(due) as DbResult<DateTime<FixedOffset>>
            })
            .unwrap_or(Ok(Local::now().fixed_offset()))?;

        Ok(Stats {
            card_count,
            review_count,
            reviews_due,
            next_review_due,
        })
    }

    /// Get the local user ID. (for now, we just have the local user, but if we ever want to turn
    /// this into a 'proper' web app, we can switch over to using an account system.)
    pub fn local_user_id() -> &'static str {
        "local"
    }

    /// Get the user record with the given ID.
    pub fn get_user_by_id(&self, user_id: &str) -> DbResult<Option<User>> {
        const QUERY: &'static str = "
            SELECT rating, rating_deviation, rating_volatility
            FROM users_v2
            WHERE id = ?
        ";

        self.conn.prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, user_id)).map_err(Self::convert_error)?
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                let rating = Rating {
                    rating: Self::try_read(&row, "rating")?,
                    deviation: Self::try_read::<i64>(&row, "rating_deviation")?,
                    volatility: Self::try_read::<f64>(&row, "rating_volatility")?,
                };

                Ok(User {
                    id: user_id.to_string(),
                    rating,
                })
            })
            .transpose()
    }

    /// Update the user record with the given ID.
    pub fn update_user(&mut self, user: &User) -> DbResult<()> {
        const QUERY: &'static str = "
            UPDATE users_v2
            SET rating = ?,
                rating_deviation = ?
            WHERE id = ?
        ";

        self.conn.prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, user.rating.rating)).map_err(Self::convert_error)?
            .bind((2, user.rating.deviation)).map_err(Self::convert_error)?
            .bind((3, user.id.as_str())).map_err(Self::convert_error)?
            .next()
            .transpose()
            .map(|_| ())
            .map_err(Self::convert_error)
    }
}
