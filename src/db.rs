use std::error::Error;
use std::fs::File;
use chrono::{DateTime, FixedOffset, Local, Duration};
use sqlite::Connection;
use owlchess::board::Board;
use owlchess::moves::Style;
use owlchess::chain::{MoveChain, NumberPolicy, GameStatusPolicy};

use crate::rating::Rating;
use crate::srs::{Card, Difficulty};

// TODO: this whole file (and project) could do with unit tests once the proof of concept is working :)

/// A result type that boxes errors to a Box<dyn Error>.
pub type DbResult<T> = Result<T, Box<dyn Error>>;

/// The puzzle database interface type.
pub struct PuzzleDatabase {
    conn: Connection,
}

/// A puzzle record from the db.
#[derive(Debug, Clone)]
pub struct Puzzle {
    pub puzzle_id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i64,
}

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

/// A review record from the db.
#[derive(Debug, Clone)]
pub struct Review {
    pub user_id: String,
    pub puzzle_id: String,
    pub difficulty: Difficulty,
    pub date: DateTime<FixedOffset>,
}

impl PuzzleDatabase {
    /// Open the given sqlite database, initialising it with schema if necessary.
    pub fn open(path: &str) -> DbResult<Self> {
        let conn = Self::init_schema(sqlite::open(path)?)?;

        Ok(Self {
            conn
        })
    }

    /// Initialise the database schema if it isn't already.
    fn init_schema(conn: Connection) -> DbResult<Connection> {
        log::info!("Initialising db schema");
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
        conn.execute(QUERY)?;
        Ok(conn)
    }

    /// Import lichess database from file.
    pub fn import_lichess_database(&mut self, lichess_db_raw: File) -> DbResult<()> {
        const MAX_PUZZLES_TO_IMPORT: usize = 10_000_000;
        const PUZZLES_PER_PROGRESS_UPDATE: usize = 10000;

        log::info!("Importing lichess puzzle database");

        if let Ok(decoder) = zstd::stream::Decoder::new(lichess_db_raw) {
            let mut csv_reader = csv::Reader::from_reader(decoder);
            let mut puzzles_imported = 0;

            let mut puzzles = Vec::new();

            for result in csv_reader.records().take(MAX_PUZZLES_TO_IMPORT) {
                const EXPECTED_ROWS: usize = 10;

                // Unwrap record.
                let record = result?;
                if record.len() != EXPECTED_ROWS {
                    log::warn!("Skipping record with {} entries, expected at least {}", record.len(), EXPECTED_ROWS);
                    continue;
                }

                // Import puzzle.
                let puzzle_id = &record[0];
                let fen = &record[1];
                let moves = &record[2];
                let rating = record[3].parse()?;

                puzzles.push(Puzzle::new(puzzle_id, fen, moves, rating));

                // Bulk insert if we have enough.
                if puzzles.len() >= PUZZLES_PER_PROGRESS_UPDATE {
                    self.add_puzzles(&puzzles)?;
                    puzzles.clear();
                }

                // Update counter and report progress.
                puzzles_imported += 1;
                if puzzles_imported % PUZZLES_PER_PROGRESS_UPDATE == 0 {
                    log::info!("Progress: {puzzles_imported} puzzles imported...");
                }
            }

            // Add last batch (should be less than the batch size or it'll be empty).
            if !puzzles.is_empty() {
                self.add_puzzles(&puzzles)?;
                puzzles.clear();
            }

            log::info!("Finished importing {puzzles_imported} puzzles");
        }

        Ok(())
    }

    /// Get the number of puzzles in the database.
    pub fn get_puzzle_count(&self) -> DbResult<usize> {
        const QUERY: &'static str = "
            SELECT COUNT(puzzle_id) FROM puzzles;
        ";

        if let Some(result) = self.conn.prepare(QUERY)?.into_iter().next() {
            Ok(result?.read::<i64, _>(0) as usize)
        }
        else {
            Ok(0)
        }
    }

    /// Add a single puzzle to the database.
    pub fn add_puzzle(&mut self, puzzle: &Puzzle) -> DbResult<()> {
        const QUERY: &'static str = "
            INSERT INTO puzzles (puzzle_id, fen, moves, rating)
            VALUES (?, ?, ?, ?)
        ";

        self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, puzzle.puzzle_id.as_str()))?
            .bind((2, puzzle.fen.as_str()))?
            .bind((3, puzzle.moves.as_str()))?
            .bind((4, puzzle.rating as i64))?
            .next();

        Ok(())
    }

    /// Add a batch of puzzles to the database.
    pub fn add_puzzles(&mut self, puzzles: &Vec<Puzzle>) -> DbResult<()> {
        // We have to build the query ourselves to do bulk insert. I'd rather use some sort of
        // batch insert api that lets you supply an iterator but I'm not sure how to do that with
        // the sqlite crate. Reusing the prepared statement was about the same overhead as just
        // creating it every time, but building the query is much faster.
        let rows = puzzles.iter().map(|puzzle|
           format!("(\"{}\", \"{}\", \"{}\", {})", puzzle.puzzle_id, puzzle.fen, puzzle.moves, puzzle.rating)
        ).collect::<Vec<_>>().join(",");

        let finished_query = format!("INSERT INTO puzzles (puzzle_id, fen, moves, rating) VALUES {}", rows);

        Ok(self.conn.execute(finished_query)?)
    }

    /// Get a puzzle by ID.
    pub fn get_puzzle_by_id(&self, puzzle_id: &str) -> DbResult<Puzzle>
    {
        const QUERY: &'static str = "
            SELECT fen, moves, rating
            FROM puzzles
            WHERE puzzle_id = ?
        ";

        let puzzle_id = puzzle_id.to_string();

        log::info!("Getting puzzle {puzzle_id}");

        let result: DbResult<Puzzle> = self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, puzzle_id.as_str()))?
            .next()
            .map(|result| {
                let row = result?;
                Ok(Puzzle::new(
                    puzzle_id.as_str(), 
                    row.read("fen"),
                    row.read("moves"),
                    row.read("rating")
                ))
            })
            .ok_or(format!("No such puzzle {puzzle_id}"))?;

            Ok(result?)
    }

    /// Get a random set of puzzles by rating.
    pub fn get_puzzles_by_rating(&self, min_rating: i64, max_rating: i64, max_puzzles: i64)
        -> DbResult<Vec<Puzzle>>
    {
        const QUERY: &'static str = "
            SELECT puzzle_id, fen, moves, rating
            FROM puzzles
            WHERE rating > ?
            AND rating < ?
            ORDER BY random()
            LIMIT ?
        ";

        log::info!("Getting puzzles..");
        let puzzles: Result<Vec<Puzzle>, sqlite::Error> = self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, min_rating as i64))?
            .bind((2, max_rating as i64))?
            .bind((3, max_puzzles as i64))?
            .map(|result| {
                let row = result?;
                Ok(Puzzle::new(
                    row.read("puzzle_id"), 
                    row.read("fen"),
                    row.read("moves"),
                    row.read("rating")
                ))
            })
            .into_iter()
            .collect();
        log::info!("Done");

        Ok(puzzles?)
    }

    /// Get the next due review.
    pub fn get_next_review_due(&self) -> DbResult<Option<(Card, Puzzle)>> {
        const QUERY: &'static str = "
            SELECT * FROM cards
            CROSS JOIN puzzles
                ON cards.puzzle_id = puzzles.puzzle_id
            WHERE due < ?
            ORDER BY due ASC
            LIMIT 1
        ";

        // Get the due cutoff time tommorow morning, so we can get all reviews due today.
        let due_time = Card::due_time()?.to_rfc3339();

        self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, due_time.as_str()))?
            .map(|result| {
                let row = result?;

                // Get puzzle id.
                let puzzle_id = row.read::<&str, _>("puzzle_id");

                // Construct card.
                let due = DateTime::parse_from_rfc3339(row.read("due"))?;
                let interval = Duration::seconds(row.read("interval"));

                let card = Card {
                    id: puzzle_id.to_string(),
                    due,
                    interval,
                    review_count: row.read("review_count"),
                    ease: row.read("ease"),
                    learning_stage: row.read("learning_stage"),
                };

                // Construct puzzle.
                let puzzle = Puzzle {
                    puzzle_id: puzzle_id.to_string(),
                    fen: row.read::<&str, _>("fen").to_string(),
                    moves: row.read::<&str, _>("moves").to_string(),
                    rating: row.read("rating"),
                };

                Ok((card, puzzle)) as DbResult<(Card, Puzzle)>
            })
            .next()
            .transpose()
    }

    /// Get a single card by ID.
    pub fn get_card_by_id(&self, puzzle_id: &str) -> DbResult<Option<Card>> {
        const QUERY: &'static str = "
            SELECT due, interval, review_count, ease, learning_stage
            FROM cards
            WHERE puzzle_id = ?
        ";

        log::info!("Getting card for puzzle {puzzle_id}");
        self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, puzzle_id))?
            .next()
            .map(|result| {
                let row = result?;

                let due = DateTime::parse_from_rfc3339(row.read("due"))?;
                let interval = Duration::seconds(row.read("interval"));

                Ok(Card {
                    id: puzzle_id.to_string(),
                    due,
                    interval,
                    review_count: row.read("review_count"),
                    ease: row.read("ease"),
                    learning_stage: row.read("learning_stage"),
                })
            })
            .transpose()
    }

    /// Update (or create) a card by ID.
    pub fn update_or_create_card(&mut self, card: &Card) -> DbResult<()> {
        const QUERY: &'static str="
            INSERT OR REPLACE INTO cards (puzzle_id, due, interval, review_count, ease, learning_stage)
            VALUES (?, ?, ?, ?, ?, ?)
        ";

        log::info!("Updating card for puzzle {}: {card:?}", card.id);

        let due = card.due.to_rfc3339();
        let interval = card.interval.num_seconds();

        Ok(self.conn
            .prepare(QUERY)?
            .into_iter()
            .bind((1, card.id.as_str()))?
            .bind((2, due.as_str()))?
            .bind((3, interval))?
            .bind((4, card.review_count as i64))?
            .bind((5, card.ease as f64))?
            .bind((6, card.learning_stage))?
            .next()
            .transpose()
            // Discard the value from the Result (which is empty anyway) and return the result.
            .map(|_| ())?)
    }

    /// Get the stats for the local user.
    pub fn get_user_stats(&self, user_id: &str) -> DbResult<Stats> {
        // For now we only support a local user, so check that it's the local user's stats that are
        // being requested. In the future, we might also want to store the user's stats in the
        // users table and just update them as needed, to avoid having to look them up every time.
        if user_id != Self::local_user_id() {
            Err(format!("get_user_stats called for non-local user {user_id}"))?
        }

        // Get card and review count.
        const QUERY: &'static str = "
            SELECT
                COUNT(*) AS card_count,
                COALESCE(SUM(review_count), 0) AS review_count
            FROM cards
        ";

        let (card_count, review_count) = self.conn
            .prepare(QUERY)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result?;

                Ok((
                    row.read::<i64, _>("card_count"),
                    row.read::<i64, _>("review_count"),
                )) as DbResult<(i64, i64)>
            })
            .unwrap_or(Ok((0, 0)))?;

        // Get the number of reviews due.
        const QUERY_2: &'static str = "
            SELECT COUNT(*) as reviews_due
            FROM cards
            WHERE date(due) < date(?)
        ";

        let due_time = Card::due_time()?.to_rfc3339();
        let reviews_due = self.conn
            .prepare(QUERY_2)?
            .into_iter()
            .bind((1, due_time.as_str()))?
            .next()
            .map(|result| {
                let row = result?;
                Ok(row.read("reviews_due")) as DbResult<i64>
            })
            .unwrap_or(Ok(0))?;

        // Get next review due time.
        const QUERY_3: &'static str = "
            SELECT due
            FROM cards
            ORDER BY due ASC
            LIMIT 1
        ";

        let next_review_due = self.conn
            .prepare(QUERY_3)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result?;
                let due = DateTime::parse_from_rfc3339(row.read("due"))?;
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

        self.conn.prepare(QUERY)?
            .into_iter()
            .bind((1, user_id))?
            .next()
            .map(|result| {
                let row = result?;

                let rating = Rating {
                    rating: row.read("rating"),
                    deviation: row.read::<i64, _>("rating_deviation"),
                    volatility: row.read::<f64, _>("rating_volatility"),
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

        self.conn.prepare(QUERY)?
            .into_iter()
            .bind((1, user.rating.rating))?
            .bind((2, user.rating.deviation))?
            .bind((3, user.id.as_str()))?
            .next()
            .transpose()?;

        Ok(())
    }

    /// Add a review record for a user.
    pub fn add_review_for_user(&mut self, review: Review) -> DbResult<()>
    {
        const QUERY: &'static str = "
            INSERT INTO reviews (user_id, puzzle_id, difficulty, date)
            VALUES (?, ?, ?, ?)
        ";

        self.conn.prepare(QUERY)?
            .into_iter()
            .bind((1, review.user_id.as_str()))?
            .bind((2, review.puzzle_id.as_str()))?
            .bind((3, review.difficulty.to_i64()))?
            .bind((4, review.date.to_rfc3339().as_str()))?
            .next()
            .transpose()?;

        Ok(())
    }

    /// Get up to the last n reviews for a user, and the rating for each one.
    pub fn last_n_reviews_for_user(&self, user_id: &str, max_reviews: i64)
        -> DbResult<Vec<(Review, i64)>>
    {
        const QUERY: &'static str = "
            SELECT reviews.puzzle_id, difficulty, date, rating
            FROM reviews
            INNER JOIN puzzles ON reviews.puzzle_id = puzzles.puzzle_id
            WHERE reviews.user_id = ?
            ORDER BY date DESC
            LIMIT ?
        ";

        self.conn.prepare(QUERY)?
            .into_iter()
            .bind((1, user_id))?
            .bind((2, max_reviews))?
            .map(|result| {
                let row = result?;

                let puzzle_id = row.read::<&str, _>("puzzle_id");
                let difficulty = Difficulty::from_i64(row.read("difficulty"))?;
                let date = DateTime::parse_from_rfc3339(row.read("date"))?;

                let review = Review {
                    user_id: user_id.to_string(),
                    puzzle_id: puzzle_id.to_string(),
                    difficulty,
                    date,
                };

                let rating = row.read("rating");

                Ok((review, rating))
            })
            .collect()
    }
}

impl Puzzle {
    /// Create a new puzzle with the given values.
    pub fn new(id: &str, fen: &str, moves: &str, rating: i64) -> Self {
        Puzzle {
            puzzle_id: id.to_string(),
            fen: fen.to_string(),
            moves: moves.to_string(),
            rating,
        }
    }

    /// Convert the embedded fen and moves from UCI format to a PGN.
    pub fn to_pgn(&self) -> Result<String, Box<dyn Error>> {
        let board = Board::from_fen(&self.fen)?;
        let movechain = MoveChain::from_uci_list(board, &self.moves)?;
        let pgn = movechain.styled(NumberPolicy::FromBoard, Style::San, GameStatusPolicy::Show).to_string();

        // Weird indentation thing so the resulting PGN doesn't have a bunch of random indentation
        // in it.
        Ok(format!("[Event \"Lichess puzzle {} (rating {})\"]
[Site \"?\"]
[Date \"????.??.??\"]
[Round \"?\"]
[White \"?\"]
[Black \"?\"]
[Result \"1-0\"]
[SetUp \"1\"]
[FEN \"{}\"]

{}", self.puzzle_id, self.rating, self.fen, pgn))
    }
}
