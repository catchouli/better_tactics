use std::fmt::Display;

use futures::TryStreamExt;
use sqlx::{Row, Sqlite, QueryBuilder, sqlite::SqliteRow};

use crate::db::{PuzzleDatabase, DbResult};

/// Puzzle id.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PuzzleId(pub i64);

impl Display for PuzzleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A puzzle record from the db.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Puzzle {
    pub id: Option<PuzzleId>,
    pub source: i64,
    pub source_id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i64,
    pub rating_deviation: i64,
    pub popularity: i64,
    pub number_of_plays: i64,
    //pub themes: Vec<String>,
    pub game_url: String,
    //pub opening_tags: Vec<String>,
}

/// A puzzle source.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PuzzleSource {
    pub id: i64,
    pub name: String,
}

/// A puzzle theme.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Theme {
    pub id: i64,
    pub name: String,
}

/// A puzzle opening.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Opening {
    pub id: i64,
    pub name: String,
}

/// Helper struct for adding puzzle themes/opening tags.
pub struct AddPuzzleTheme {
    pub source: i64,
    pub source_id: String,
    pub theme_id: i64,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Puzzle
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: Some(PuzzleId(row.try_get("id")?)),
            source: row.try_get("source")?,
            source_id: row.try_get("source_id")?,
            fen: row.try_get("fen")?,
            moves: row.try_get("moves")?,
            rating: row.try_get("rating")?,
            rating_deviation: row.try_get("rating_deviation")?,
            popularity: row.try_get("popularity")?,
            number_of_plays: row.try_get("number_of_plays")?,
            //themes: row.try_get::<String, _>("themes")?
            //    .split_whitespace().map(ToString::to_string).collect(),
            game_url: row.try_get("game_url")?,
            //opening_tags: row.try_get::<String, _>("opening_tags")?
            //    .split_whitespace().map(ToString::to_string).collect(),
        })
    }
}

/// Puzzle related database implementations.
impl PuzzleDatabase {
    /// Get the number of puzzles in the database.
    pub async fn get_puzzle_count(&self) -> DbResult<usize> {
        let query = sqlx::query("
            SELECT count(id) as puzzle_count FROM puzzles;
        ");

        query
            .map(|row: SqliteRow| {
                Ok(row.try_get::<i64, _>("puzzle_count")? as usize)
            })
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or(Ok(0))
    }

    /// Get the lowest puzzle rating.
    pub async fn get_min_puzzle_rating(&self) -> DbResult<i64> {
        // https://stackoverflow.com/questions/11515165/sqlite3-select-min-max-together-is-much-slower-than-select-them-separately
        Ok(sqlx::query("SELECT min(rating) AS rating FROM puzzles")
            .map(|row: SqliteRow| row.try_get::<i64, _>("rating"))
            .fetch_optional(&self.pool)
            .await?
            .transpose()?
            .unwrap_or(0))
    }

    /// Get the highest puzzle rating.
    pub async fn get_max_puzzle_rating(&self) -> DbResult<i64> {
        // https://stackoverflow.com/questions/11515165/sqlite3-select-min-max-together-is-much-slower-than-select-them-separately
        Ok(sqlx::query("SELECT max(rating) AS rating FROM puzzles")
            .map(|row: SqliteRow| row.try_get::<i64, _>("rating"))
            .fetch_optional(&self.pool)
            .await?
            .transpose()?
            .unwrap_or(0))
    }

    /// Add a batch of puzzles to the database.
    pub async fn add_puzzles(&mut self, puzzles: &Vec<Puzzle>) -> DbResult<()> {
        const BATCH_SIZE: usize = 500;

        let mut conn = self.pool.begin().await?;

        sqlx::query("
            CREATE TEMPORARY TABLE IF NOT EXISTS lichess_puzzles (
                source INTEGER,
                source_id TEXT,
                fen TEXT,
                moves TEXT,
                rating INTEGER,
                rating_deviation INTEGER,
                popularity INTEGER,
                number_of_plays INTEGER,
                game_url TEXT
            );
        ").execute(&mut *conn).await?;

        // We have to build the query ourselves to do bulk insert. I'd rather use some sort of
        // batch insert api that lets you supply an iterator but I'm not sure how to do that with
        // the sqlite crate. Reusing the prepared statement was about the same overhead as just
        // creating it every time, but building the query is much faster.
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO lichess_puzzles (source, source_id, fen, moves, rating,
            rating_deviation, popularity, number_of_plays, game_url) "
        );

        for batch in puzzles.chunks(BATCH_SIZE) {
            query_builder.reset();

            query_builder.push_values(batch, |mut b, puzzle| {
                b.push_bind(&puzzle.source)
                    .push_bind(&puzzle.source_id)
                    .push_bind(&puzzle.fen)
                    .push_bind(&puzzle.moves)
                    .push_bind(puzzle.rating)
                    .push_bind(puzzle.rating_deviation)
                    .push_bind(puzzle.popularity)
                    .push_bind(puzzle.number_of_plays)
                    .push_bind(&puzzle.game_url);
            });

            query_builder
                .build()
                .execute(&mut *conn)
                .await?;
        }

        sqlx::query("
            INSERT OR REPLACE INTO puzzles (source, source_id, fen, moves, rating, rating_deviation,
                popularity, number_of_plays, game_url)
            SELECT source, source_id, fen, moves, rating, rating_deviation, popularity, number_of_plays,
                game_url
            FROM lichess_puzzles
            WHERE true
            -- Update values that might change if it already exists.
            ON CONFLICT(source, source_id)
            DO UPDATE
            SET rating = excluded.rating,
                rating_deviation = excluded.rating_deviation,
                popularity = excluded.popularity,
                number_of_plays = excluded.number_of_plays,
                game_url = excluded.game_url;

            DELETE FROM lichess_puzzles;
        ").execute(&mut *conn).await?;

        conn.commit().await?;

        Ok(())
    }

    /// Get a puzzle by ID.
    pub async fn get_puzzle_by_id(&self, id: PuzzleId) -> DbResult<Option<Puzzle>>
    {
        log::info!("Getting puzzle {id}");

        let query = sqlx::query_as("
            SELECT *
            FROM puzzles
            WHERE id = ?
        ");

        Ok(query
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Get a puzzle by ID.
    pub async fn get_puzzle_by_source_id(&self, source_name: &str, source_id: &str) -> DbResult<Option<Puzzle>>
    {
        log::info!("Getting puzzle {source_name}/{source_id}");

        let query = sqlx::query_as("
            SELECT * FROM puzzles
            WHERE source_id = ?
            AND source = (SELECT id FROM puzzle_sources WHERE name = ?)
            LIMIT 1
        ");

        Ok(query
            .bind(source_id)
            .bind(source_name)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// Get a random set of puzzles by rating.
    pub async fn get_puzzles_by_rating(&self, min_rating: i64, max_rating: i64, max_puzzles: i64)
        -> DbResult<Vec<Puzzle>>
    {
        log::info!("Getting puzzles..");

        let query = sqlx::query_as("
            SELECT *
            FROM puzzles
            WHERE rating >= ?
            AND rating <= ?
            ORDER BY random()
            LIMIT ?");

        Ok(query
            .bind(min_rating)
            .bind(max_rating)
            .bind(max_puzzles)
            .fetch(&self.pool)
            .try_collect()
            .await?)
    }

    /// Get a puzzle source.
    pub async fn get_puzzle_source(&self, name: &str) -> DbResult<Option<PuzzleSource>> {
        let query = sqlx::query_as("
            SELECT *
            FROM puzzle_sources
            WHERE name = ?
        ");

        Ok(query
           .bind(name)
           .fetch_optional(&self.pool)
           .await?)
    }

    // Get a puzzle theme.
    pub async fn get_theme(&self, name: &str) -> DbResult<Option<Theme>> {
        let query = sqlx::query_as("
            SELECT *
            FROM themes
            WHERE name = ?
        ");

        Ok(query
           .bind(name)
           .fetch_optional(&self.pool)
           .await?)
    }

    // Add a puzzle theme.
    pub async fn add_theme(&self, name: &str) -> DbResult<()> {
        let query = sqlx::query("
            INSERT INTO themes (name) VALUES (?)
        ");

        query
           .bind(name)
           .execute(&self.pool)
           .await?;

        Ok(())
    }

    // Get a puzzle opening.
    pub async fn get_opening(&self, name: &str) -> DbResult<Option<Opening>> {
        let query = sqlx::query_as("
            SELECT *
            FROM openings
            WHERE name = ?
        ");

        Ok(query
           .bind(name)
           .fetch_optional(&self.pool)
           .await?)
    }

    // Add a puzzle opening.
    pub async fn add_opening(&self, name: &str) -> DbResult<()> {
        let query = sqlx::query("
            INSERT INTO openings (name) VALUES (?)
        ");

        query
           .bind(name)
           .execute(&self.pool)
           .await?;

        Ok(())
    }

    /// Add theme tags to a puzzle.
    pub async fn add_puzzle_themes(&self, themes: &Vec<AddPuzzleTheme>) -> DbResult<()> {
        let mut conn = self.pool.begin().await?;

        sqlx::query("
            CREATE TEMPORARY TABLE IF NOT EXISTS puzzle_themes_temp (
                source INTEGER,
                source_id TEXT,
                theme_id INTEGER
            );
        ").execute(&mut *conn).await?;

        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO puzzle_themes_temp (source, source_id, theme_id) "
        );

        for themes in themes.chunks(1000) {
            query_builder.reset();

            query_builder.push_values(themes, |mut b, entry| {
                b.push_bind(&entry.source)
                    .push_bind(&entry.source_id)
                    .push_bind(&entry.theme_id);
            });

            query_builder
                .build()
                .execute(&mut *conn)
                .await?;
        }

        sqlx::query("
            INSERT OR REPLACE INTO puzzle_themes
            SELECT puzzles.id AS puzzle_id, theme_id FROM puzzle_themes_temp
            JOIN puzzles
            ON puzzles.source = puzzle_themes_temp.source
            AND puzzles.source_id = puzzle_themes_temp.source_id;

            DELETE FROM puzzle_themes_temp;
        ").execute(&mut *conn).await?;

        conn.commit().await?;

        Ok(())
    }

    /// Add opening tags to a puzzle.
    pub async fn add_puzzle_openings(&self, openings: &[AddPuzzleTheme]) -> DbResult<()> {
        let mut conn = self.pool.begin().await?;

        sqlx::query("
            CREATE TEMPORARY TABLE IF NOT EXISTS puzzle_openings_temp (
                source INTEGER,
                source_id TEXT,
                theme_id INTEGER
            );
        ").execute(&mut *conn).await?;

        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO puzzle_openings_temp (source, source_id, theme_id) "
        );

        for openings in openings.chunks(1000) {
            query_builder.reset();

            query_builder.push_values(openings, |mut b, entry| {
                b.push_bind(&entry.source)
                    .push_bind(&entry.source_id)
                    .push_bind(&entry.theme_id);
            });

            query_builder
                .build()
                .execute(&mut *conn)
                .await?;
        }

        sqlx::query("
            INSERT OR REPLACE INTO puzzle_openings
            SELECT puzzles.id AS puzzle_id, theme_id FROM puzzle_openings_temp
            JOIN puzzles
            ON puzzles.source = puzzle_openings_temp.source
            AND puzzles.source_id = puzzle_openings_temp.source_id;

            DELETE FROM puzzle_openings_temp;
        ").execute(&mut *conn).await?;

        conn.commit().await?;

        Ok(())
    }
}
