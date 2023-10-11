use sqlx::sqlite::*;
use sqlx::Row;

use crate::rating::Rating;
use crate::db::{PuzzleDatabase, DbResult};

/// A stats record from the db (for the local user, for now).


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
