use std::fmt::Display;
use std::path::Path;

use sqlx::{Sqlite, Executor};

use crate::db::{PuzzleDatabase, DbResult, DatabaseError, ErrorDetails};

impl PuzzleDatabase {
    // Back up the database (sans puzzles) to the given file.
    pub async fn backup_database(&self, path: &str) -> DbResult<()> {
        log::info!("Backing up database to {path}");

        // Create the directory if it doesn't already exist.
        if let Some(directory) = Path::new(path).parent() {
            if !directory.exists() {
                log::info!("Creating backup directory {}", directory.as_os_str().to_string_lossy());
                std::fs::create_dir_all(directory)
                    .map_err(Self::backup_error)?;
            }
        }

        // Create database file.
        self.create_backup_db(path).await
            .map_err(Self::backup_error)?;

        // Use the same connection from the pool or we might get one the backup database isn't
        // attached to.
        let mut conn = self.pool.acquire().await?;

        // Attach backup database.
        Self::attach_backup_db(&mut *conn, path).await
            .map_err(Self::backup_error)?;

        // Back up tables.
        let backup_result = Self::backup_tables(&mut *conn).await;

        // Detach db regardless of whether the backup succeeded.
        let detach_result = Self::detach_backup_db(&mut *conn).await;

        backup_result.and(detach_result)
    }

    async fn create_backup_db(&self, path: &str) -> DbResult<()> {
        log::info!("Creating backup database");
        PuzzleDatabase::open(path, self.srs_config).await?;
        Ok(())
    }

    async fn attach_backup_db<'a, E>(conn: E, path: &str) -> DbResult<()>
        where E: Executor<'a, Database = Sqlite>
    {
        log::info!("Attaching backup database");
        sqlx::query("ATTACH ? AS backup_db")
            .bind(&path)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn detach_backup_db<'a, E>(conn: E) -> DbResult<()>
        where E: Executor<'a, Database = Sqlite>
    {
        log::info!("Detaching backup database");
        sqlx::query("DETACH backup_db")
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn backup_tables<'a, E>(conn: E) -> DbResult<()>
        where E: Executor<'a, Database = Sqlite>
    {
        log::info!("Backing up tables");

        // Back up all tables *except* the puzzles table, which is quite big and can just be
        // imported again at next start.
        let query = sqlx::query("
            INSERT OR REPLACE INTO backup_db.app_data
            SELECT * FROM app_data;

            INSERT OR REPLACE INTO backup_db.cards
            SELECT * FROM cards;

            INSERT OR REPLACE INTO backup_db.reviews
            SELECT * FROM reviews;

            INSERT OR REPLACE INTO backup_db.skipped_puzzles
            SELECT * FROM skipped_puzzles;

            INSERT OR REPLACE INTO backup_db.users
            SELECT * FROM users;

            UPDATE backup_db.app_data
            SET lichess_db_imported=0;
        ");

        query.execute(conn).await?;

        Ok(())
    }

    fn backup_error<T: Display>(e: T) -> DatabaseError {
        DatabaseError::BackupError(ErrorDetails {
            backend: "sqlite".into(),
            description: format!("Error when backing up database file: {}", e),
            source: None,
        })
    }
}
