use std::fmt::Display;

use chrono::DateTime;
use chrono::FixedOffset;
use sqlx::sqlite::*;
use sqlx::Row;

use crate::rating::Rating;
use crate::db::{PuzzleDatabase, DbResult};

use super::DatabaseError;
use super::ErrorDetails;
use super::PuzzleId;

/// User id.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UserId(pub i64);

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A user record from the db.
#[derive(Debug, Clone)]
pub struct User {
    pub id: Option<UserId>,
    pub username: String,
    pub rating: Rating,
    pub next_puzzle: Option<PuzzleId>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for User
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: Some(UserId(row.try_get("id")?)),
            username: row.try_get("username")?,
            rating: Rating {
                // TODO: these should be optional, and run the rating wizard if rating is NULL.
                rating: row.try_get::<Option<i64>, _>("rating")?.unwrap_or(500),
                deviation: row.try_get::<Option<i64>, _>("rating_deviation")?.unwrap_or(250),
                volatility: row.try_get::<Option<f64>, _>("rating_volatility")?.unwrap_or(0.06),
            },
            next_puzzle: row.try_get::<Option<i64>, _>("next_puzzle")?.map(|i| PuzzleId(i)),
        })
    }
}

impl PuzzleDatabase {
    /// Get the user id of the user with the given username.
    pub async fn get_user_id_by_username(&self, username: &str) -> DbResult<Option<UserId>> {
        let query = sqlx::query("
            SELECT id
            FROM users
            WHERE username = ?
        ");

        query
            .bind(username)
            .map(|a: SqliteRow| Ok(UserId(a.try_get("id")?)))
            .fetch_optional(&self.pool)
            .await?
            .transpose()
    }

    /// Get the user record with the given ID.
    pub async fn get_user_by_id(&self, user_id: UserId) -> DbResult<Option<User>> {
        sqlx::query_as("
            SELECT *
            FROM users
            WHERE id = ?
        ")
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await.map_err(Into::into)
    }

    /// Update the user record with the given ID.
    pub async fn update_user(&mut self, user: &User) -> DbResult<()> {
        let user_id = user.id
            .ok_or_else(|| DatabaseError::QueryError(ErrorDetails {
                description: format!("update_user called for user with no id"),
                backend: "sqlite".into(),
                source: None,
            }))?;

        sqlx::query("
            UPDATE users
            SET username = ?,
                rating = ?,
                rating_deviation = ?,
                rating_volatility = ?,
                next_puzzle = ?
            WHERE id = ?
        ")
        .bind(&user.username)
        .bind(user.rating.rating)
        .bind(user.rating.deviation)
        .bind(user.rating.volatility)
        .bind(user.next_puzzle.map(|p| p.0))
        .bind(user_id.0)
        .execute(&self.pool)
        .await.map(|_| ()).map_err(Into::into)
    }

    /// Add a puzzle to the skipped puzzles list for a user.
    pub async fn add_skipped_puzzle(&mut self, user_id: UserId, puzzle_id: PuzzleId,
                                    dt: DateTime<FixedOffset>) -> DbResult<()>
    {
        sqlx::query("INSERT INTO skipped_puzzles (user_id, puzzle_id, date) VALUES (?, ?, ?)")
            .bind(user_id.0)
            .bind(puzzle_id.0)
            .bind(&dt.to_rfc3339())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
