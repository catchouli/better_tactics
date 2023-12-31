use futures::TryStreamExt;
use sqlx::{Row, Sqlite, QueryBuilder, sqlite::SqliteRow};

use crate::db::{PuzzleDatabase, DbResult};

/// A puzzle record from the db.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Puzzle {
    pub puzzle_id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i64,
    pub rating_deviation: i64,
    pub popularity: i64,
    pub number_of_plays: i64,
    pub themes: Vec<String>,
    pub game_url: String,
    pub opening_tags: Vec<String>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Puzzle
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            puzzle_id: row.try_get("puzzle_id")?,
            fen: row.try_get("fen")?,
            moves: row.try_get("moves")?,
            rating: row.try_get("rating")?,
            rating_deviation: row.try_get("rating_deviation")?,
            popularity: row.try_get("popularity")?,
            number_of_plays: row.try_get("number_of_plays")?,
            themes: row.try_get::<String, _>("themes")?
                .split_whitespace().map(ToString::to_string).collect(),
            game_url: row.try_get("game_url")?,
            opening_tags: row.try_get::<String, _>("opening_tags")?
                .split_whitespace().map(ToString::to_string).collect(),
        })
    }
}

/// Puzzle related database implementations.
impl PuzzleDatabase {
    /// Get the number of puzzles in the database.
    pub async fn get_puzzle_count(&self) -> DbResult<usize> {
        let query = sqlx::query("
            SELECT count(puzzle_id) as puzzle_count FROM puzzles;
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
                puzzle_id TEXT,
                fen TEXT,
                moves TEXT,
                rating INTEGER,
                rating_deviation INTEGER,
                popularity INTEGER,
                number_of_plays INTEGER,
                themes TEXT,
                game_url TEXT,
                opening_tags TEXT
            );
        ").execute(&mut *conn).await?;

        // We have to build the query ourselves to do bulk insert. I'd rather use some sort of
        // batch insert api that lets you supply an iterator but I'm not sure how to do that with
        // the sqlite crate. Reusing the prepared statement was about the same overhead as just
        // creating it every time, but building the query is much faster.
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO lichess_puzzles (puzzle_id, fen, moves, rating, rating_deviation,
            popularity, number_of_plays, themes, game_url, opening_tags) "
            );

        for batch in puzzles.chunks(BATCH_SIZE) {
            query_builder.reset();

            query_builder.push_values(batch, |mut b, puzzle| {
                b.push_bind(&puzzle.puzzle_id)
                    .push_bind(&puzzle.fen)
                    .push_bind(&puzzle.moves)
                    .push_bind(puzzle.rating)
                    .push_bind(puzzle.rating_deviation)
                    .push_bind(puzzle.popularity)
                    .push_bind(puzzle.number_of_plays)
                    .push_bind(puzzle.themes.join(" "))
                    .push_bind(&puzzle.game_url)
                    .push_bind(puzzle.opening_tags.join(" "));
            });

            query_builder
                .build()
                .execute(&mut *conn)
                .await?;
        }

        sqlx::query("
            INSERT OR REPLACE INTO puzzles
            SELECT * FROM lichess_puzzles;

            DELETE FROM lichess_puzzles;
        ").execute(&mut *conn).await?;

        conn.commit().await?;

        Ok(())
    }

    /// Get a puzzle by ID.
    pub async fn get_puzzle_by_id(&self, puzzle_id: &str) -> DbResult<Option<Puzzle>>
    {
        log::info!("Getting puzzle {puzzle_id}");

        let query = sqlx::query_as("
            SELECT *
            FROM puzzles
            WHERE puzzle_id = ?
        ");

        Ok(query
            .bind(puzzle_id)
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
}
