mod dbresult;
mod puzzle;
mod user;
mod card;
mod migration;
mod backup;

use chrono::{DateTime, FixedOffset};
pub use dbresult::*;
pub use puzzle::*;
pub use user::*;
pub use card::*;

use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions, SqliteRow};
use sqlx::{SqlitePool, ConnectOptions, Row};
use url::Url;

use crate::srs::SrsConfig;

/// The puzzle database interface type.
#[derive(Clone)]
pub struct PuzzleDatabase {
    pool: SqlitePool,
    srs_config: SrsConfig,
}

pub struct AppData {
    pub environment: String,
    pub lichess_db_imported: bool,
    pub last_backup_date: Option<DateTime<FixedOffset>>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for AppData
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            environment: row.try_get("environment")?,
            lichess_db_imported: row.try_get::<i64, _>("lichess_db_imported")? != 0,
            last_backup_date: row.try_get::<Option<&str>, _>("last_backup_date")?
                .map(DateTime::parse_from_rfc3339)
                .transpose()
                .map_err(|e| sqlx::Error::ColumnDecode {
                    index: "date".to_string(),
                    source: e.to_string().into(),
                })?,
        })
    }
}

impl PuzzleDatabase {
    /// Open the given sqlite database, initialising it with schema if necessary.
    pub async fn open(url: &Url, srs_config: SrsConfig) -> DbResult<Self> {
        // Open sqlite database.
        // TODO: we aren't really making use of the database pools right now because we have a
        // single PuzzleDatabase instance behind a mutex.
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(SqliteConnectOptions::from_url(url)?
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

        // Enable write-ahead-logging (single writer multiple readers), and set the busy timeout to
        // 30 seconds so that requests don't fail just because something is already trying to write
        // to the database;
        sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
        sqlx::query("PRAGMA busy_timeout=30000").execute(&pool).await?;

        Ok(Self {
            pool,
            srs_config,
        })
    }

    /// Force an sqlite checkpoint and close the database.
    pub async fn close(&self) -> DbResult<()> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await?;
        self.pool.close().await;
        Ok(())
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
            INSERT OR REPLACE INTO app_data (environment, lichess_db_imported, last_backup_date)
            VALUES (?, ?, ?)
        ")
        .bind(&app_data.environment)
        .bind(&app_data.lichess_db_imported)
        .bind(&app_data.last_backup_date.as_ref().map(DateTime::to_rfc3339))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

