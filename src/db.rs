mod dbresult;
mod puzzle;
mod user;
mod card;

use std::str::FromStr;

use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};
use sqlx::{SqlitePool, ConnectOptions};

pub use dbresult::*;
pub use puzzle::*;
pub use user::*;
pub use card::*;

// TODO: this whole file (and project) could do with unit tests once the proof of concept is working :)
// Also, we should probably just switch to diesel now it's up and running, and it would solve the
// problems of data migration and the general ad-hoc-ness of it all.

const DB_VERSION: i64 = 1;

/// The puzzle database interface type.
pub struct PuzzleDatabase {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct AppData {
    pub environment: String,
    pub db_version: i64,
    pub lichess_db_imported: bool,
}

impl PuzzleDatabase {
    /// Open the given sqlite database, initialising it with schema if necessary.
    pub async fn open(path: &str) -> DbResult<Self> {
        // Open sqlite database.
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(SqliteConnectOptions::from_str(path)?
              .disable_statement_logging()
            )
            .await
            .map_err(|e| DatabaseError::ConnectionError(ErrorDetails {
                backend: format!("sqlite"),
                description: format!("Database connection error: {e}"),
                source: Some(e.into()),
            }))?;

        // Initialise schema if it isn't already.
        Self::init_schema(&pool).await?;

        Ok(Self {
            pool
        })
    }

    /// Initialise the database schema if it isn't already.
    /// TODO: take a generic connection.
    async fn init_schema(pool: &SqlitePool) -> DbResult<()> {
        log::info!("Initialising db schema");
        let query = format!("
            CREATE TABLE IF NOT EXISTS puzzles (
                puzzle_id TEXT PRIMARY KEY,
                fen TEXT NOT NULL,
                moves TEXT NOT NULL,
                rating INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cards (
                puzzle_id TEXT PRIMARY KEY,
                due TEXT NOT NULL,
                interval INTEGER NOT NULL,
                review_count INTEGER NOT NULL,
                ease FLOAT NOT NULL,
                learning_stage INTEGER NOT NULL
            );
            DROP TABLE IF EXISTS users;
            CREATE TABLE IF NOT EXISTS users_v2 (
                id TEXT PRIMARY KEY,
                rating INTEGER NOT NULL,
                rating_deviation INTEGER NOT NULL,
                rating_volatility FLOAT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS reviews (
                user_id TEXT NOT NULL,
                puzzle_id TEXT NOT NULL,
                difficulty INTEGER NOT NULL,
                date TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_data (
                environment TEXT PRIMARY KEY,
                db_version INTEGER NOT NULL,
                lichess_db_imported BOOLEAN NOT NULL
            );
            INSERT OR IGNORE INTO users_v2 (id, rating, rating_deviation, rating_volatility)
                VALUES ('local', 500, 250, 0.06);
            INSERT OR IGNORE INTO app_data (environment, db_version, lichess_db_imported)
                VALUES ('', {DB_VERSION}, 0);
            CREATE INDEX IF NOT EXISTS user_id ON users_v2(id);
            CREATE INDEX IF NOT EXISTS card_id ON cards(puzzle_id);
            CREATE INDEX IF NOT EXISTS puzzle_id ON puzzles(puzzle_id);
            CREATE INDEX IF NOT EXISTS puzzle_rating ON puzzles(rating);
        ");

        sqlx::query(&query)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_app_data(&self, env: &str) -> DbResult<AppData> {
        Ok(sqlx::query_as("
                SELECT *
                FROM app_data
                WHERE environment = ?
            ")
            .bind(env)
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn set_app_data(&self, app_data: &AppData) -> DbResult<()> {
        sqlx::query("
            INSERT OR REPLACE INTO app_data (environment, db_version, lichess_db_imported)
            VALUES (?, ?, ?)
        ")
        .bind(&app_data.environment)
        .bind(app_data.db_version)
        .bind(app_data.lichess_db_imported)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

