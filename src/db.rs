mod dbresult;
mod puzzle;
mod user;
mod card;

use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;
use chrono::{DateTime, FixedOffset};
use sqlite::{Connection, Row, Value};

pub use dbresult::*;
pub use puzzle::*;
pub use user::*;
pub use card::*;

// TODO: this whole file (and project) could do with unit tests once the proof of concept is working :)
// Also, we should probably just switch to diesel now it's up and running, and it would solve the
// problems of data migration and the general ad-hoc-ness of it all.

const DB_BACKEND: &'static str = "Sqlite";

/// The puzzle database interface type.
pub struct PuzzleDatabase {
    conn: Connection,
}

impl PuzzleDatabase {
    /// Open the given sqlite database, initialising it with schema if necessary.
    pub fn open(path: &str) -> DbResult<Self> {
        // Open sqlite database.
        let mut conn = sqlite::open(path)
            .map_err(|e| DatabaseError::ConnectionError(ErrorDetails {
                backend: DB_BACKEND.to_string(),
                description: format!("Connection error: {e}"),
                source: Some(e.into()),
            }))?;

        // Initialise schema if it isn't already.
        Self::init_schema(&mut conn)?;

        Ok(Self {
            conn
        })
    }

    /// Initialise the database schema if it isn't already.
    fn init_schema(conn: &mut Connection) -> DbResult<()> {
        log::info!("Initialising db schema");
        // TODO: make this fail on purpose and check the error is reported correctly.
        const QUERY: &'static str = "
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
            INSERT OR IGNORE INTO users_v2 (id, rating, rating_deviation, rating_volatility)
                VALUES ('local', 500, 250, 0.06);
            CREATE INDEX IF NOT EXISTS user_id ON users_v2(id);
            CREATE INDEX IF NOT EXISTS card_id ON cards(puzzle_id);
            CREATE INDEX IF NOT EXISTS puzzle_id ON puzzles(puzzle_id);
            CREATE INDEX IF NOT EXISTS puzzle_rating ON puzzles(rating);
        ";

        conn.execute(QUERY)
            .map_err(|e| DatabaseError::QueryError(ErrorDetails {
                backend: DB_BACKEND.to_string(),
                description: format!("Failed to initialise database schema: {e}"),
                source: Some(e.into()),
            }))
    }

    /// A wrapper for sqlite's Row::try_read() that converts errors to our DatabaseError type and
    /// allows us to handle and report them easily.
    fn try_read<'l, T>(row: &'l Row, column: &str) -> DbResult<T>
    where
        T: TryFrom<&'l Value, Error = sqlite::Error>,
    { 
        row.try_read::<T, _>(column)
            .map_err(|e: sqlite::Error| {
                let message = e.message.as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("(no description)");

                DatabaseError::QueryError(ErrorDetails {
                    backend: DB_BACKEND.to_string(),
                    description: format!("When reading row {:?}: {}", column, message),
                    source: Some(e.into()),
                }).into()
            })
    }

    /// A wrapper for String::parse that converts errors to our DatabaseError type and allows us to
    /// handle and report them easily.
    fn try_parse<T>(value: &str) -> DbResult<T>
    where
        T: FromStr,
        T::Err: Into<Box<dyn Error>> + Display,
    {
        value.parse::<T>()
            .map_err(|e| DatabaseError::ParsingError(ErrorDetails {
                backend: "String".to_string(),
                description: format!("Failed to parse value: {e}"),
                source: Some(e.into()),
            }))
    }

    /// Parse a datetime from a rfc3339 format string and return a ParsingError if it failed.
    fn try_parse_datetime(value: &str) -> DbResult<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(value)
            .map_err(|e| DatabaseError::ParsingError(ErrorDetails {
                backend: "chrono".to_string(),
                source: Some(e.into()),
                description: format!("Failed to parse datetime \"{value}\": {e}"),
            }))
    }

    /// A generic way of wrapping sqlite::Error to DatabaseError for when we don't need more direct
    /// control.
    fn convert_error(err: sqlite::Error) -> DatabaseError {
        DatabaseError::ParsingError(ErrorDetails {
            backend: DB_BACKEND.to_string(),
            description: err.to_string(),
            source: Some(err.into()),
        })
    }
}

