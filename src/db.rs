mod dbresult;
mod puzzle;
mod user;
mod card;
mod migration;

use std::str::FromStr;

use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};
use sqlx::{SqlitePool, ConnectOptions};

pub use dbresult::*;
pub use puzzle::*;
pub use user::*;
pub use card::*;

use crate::config::SrsConfig;

// TODO: this whole file (and project) could do with unit tests once the proof of concept is working :)

/// The puzzle database interface type.
pub struct PuzzleDatabase {
    pool: SqlitePool,
    srs_config: SrsConfig,
}

#[derive(sqlx::FromRow)]
pub struct AppData {
    pub environment: String,
    pub lichess_db_imported: bool,
}

impl PuzzleDatabase {
    /// Open the given sqlite database, initialising it with schema if necessary.
    pub async fn open(path: &str, srs_config: SrsConfig) -> DbResult<Self> {
        // Open sqlite database.
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(SqliteConnectOptions::from_str(path)?
              .disable_statement_logging()
              .create_if_missing(true)
            )
            .await
            .map_err(|e| DatabaseError::ConnectionError(ErrorDetails {
                backend: format!("sqlite"),
                description: format!("Database connection error: {e}"),
                source: Some(e.into()),
            }))?;

        // Run migrations.
        log::info!("Running database migrations...");
        sqlx::migrate!().run(&pool).await?;

        Ok(Self {
            pool,
            srs_config,
        })
    }

    pub async fn get_app_data(&self, env: &str) -> DbResult<Option<AppData>> {
        Ok(sqlx::query_as("
                SELECT *
                FROM app_data
                WHERE environment = ?
            ")
            .bind(env)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn set_app_data(&self, app_data: &AppData) -> DbResult<()> {
        sqlx::query("
            INSERT OR REPLACE INTO app_data (environment, lichess_db_imported)
            VALUES (?, ?)
        ")
        .bind(&app_data.environment)
        .bind(app_data.lichess_db_imported)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

