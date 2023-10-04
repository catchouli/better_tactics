use chrono::{DateTime, FixedOffset, Duration};

use crate::srs::{Card, Difficulty};
use crate::db::{PuzzleDatabase, DbResult, Puzzle, DatabaseError, ErrorDetails};

/// A review record from the db.
#[derive(Debug, Clone)]
pub struct Review {
    pub user_id: String,
    pub puzzle_id: String,
    pub difficulty: Difficulty,
    pub date: DateTime<FixedOffset>,
}

impl PuzzleDatabase {
    /// Get the next due review. min_interval allows us to filter out cards with short intervals
    /// (e.g. because they're still in learning), because otherwise they'll show up, possibly
    /// repeatedly if learning or relearning, before other cards that are due later today.
    pub fn get_next_review_due(&self, time: DateTime<FixedOffset>, min_interval: Option<Duration>)
        -> DbResult<Option<(Card, Puzzle)>>
    {
        // TODO: the left join means that if the corresponding puzzle gets deleted, the query will
        // return NULL for those fields, and the try_reads below will fail. For now, I'm not really
        // expecting the puzzle database itself to change after it's initially imported, but we
        // should probably enforce data integrity with foreign key constraints, or just handle the
        // case when the puzzle data has changed in some way.
        const QUERY: &'static str = "
            SELECT * FROM cards
            LEFT JOIN puzzles
                ON cards.puzzle_id = puzzles.puzzle_id
            WHERE datetime(due) < datetime(?)
            AND interval >= ?
            ORDER BY datetime(due) ASC
            LIMIT 1
        ";

        let min_interval_seconds = min_interval.map(|i| i.num_seconds()).unwrap_or(0);

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, time.to_rfc3339().as_str())).map_err(Self::convert_error)?
            .bind((2, min_interval_seconds)).map_err(Self::convert_error)?
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                // Get puzzle id.
                let puzzle_id = Self::try_read::<&str>(&row, "puzzle_id")?;

                // Construct card.
                let due = Self::try_parse_datetime(Self::try_read(&row, "due")?)?;
                let interval = Duration::seconds(Self::try_read(&row, "interval")?);

                let card = Card {
                    id: puzzle_id.to_string(),
                    due,
                    interval,
                    review_count: Self::try_read(&row, "review_count")?,
                    ease: Self::try_read(&row, "ease")?,
                    learning_stage: Self::try_read(&row, "learning_stage")?,
                };

                // Construct puzzle.
                let puzzle = Puzzle {
                    puzzle_id: puzzle_id.to_string(),
                    fen: Self::try_read::<&str>(&row, "fen")?.to_string(),
                    moves: Self::try_read::<&str>(&row, "moves")?.to_string(),
                    rating: Self::try_read(&row, "rating")?,
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
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, puzzle_id)).map_err(Self::convert_error)?
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                let due = Self::try_parse_datetime(Self::try_read(&row, "due")?)?;
                let interval = Duration::seconds(Self::try_read(&row, "interval")?);

                Ok(Card {
                    id: puzzle_id.to_string(),
                    due,
                    interval,
                    review_count: Self::try_read(&row, "review_count")?,
                    ease: Self::try_read(&row, "ease")?,
                    learning_stage: Self::try_read(&row, "learning_stage")?,
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

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, card.id.as_str())).map_err(Self::convert_error)?
            .bind((2, due.as_str())).map_err(Self::convert_error)?
            .bind((3, interval)).map_err(Self::convert_error)?
            .bind((4, card.review_count as i64)).map_err(Self::convert_error)?
            .bind((5, card.ease as f64)).map_err(Self::convert_error)?
            .bind((6, card.learning_stage)).map_err(Self::convert_error)?
            .next()
            .transpose()
            .map(|_| ())
            .map_err(Self::convert_error)
    }

    /// Add a review record for a user.
    pub fn add_review_for_user(&mut self, review: Review) -> DbResult<()>
    {
        const QUERY: &'static str = "
            INSERT INTO reviews (user_id, puzzle_id, difficulty, date)
            VALUES (?, ?, ?, ?)
        ";

        self.conn.prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, review.user_id.as_str())).map_err(Self::convert_error)?
            .bind((2, review.puzzle_id.as_str())).map_err(Self::convert_error)?
            .bind((3, review.difficulty.to_i64())).map_err(Self::convert_error)?
            .bind((4, review.date.to_rfc3339().as_str())).map_err(Self::convert_error)?
            .next()
            .transpose()
            .map(|_| ())
            .map_err(Self::convert_error)
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

        self.conn.prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, user_id)).map_err(Self::convert_error)?
            .bind((2, max_reviews)).map_err(Self::convert_error)?
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                let puzzle_id = Self::try_read::<&str>(&row, "puzzle_id")?;
                let date = Self::try_parse_datetime(Self::try_read(&row, "date")?)?;

                let difficulty = Difficulty::from_i64(Self::try_read(&row, "difficulty")?)
                    .map_err(|e| DatabaseError::ParsingError(ErrorDetails {
                        backend: "Difficulty".to_string(),
                        description: format!("Failed to parse difficulty value: {e}"),
                        source: Some(e.into()),
                    }))?;

                let review = Review {
                    user_id: user_id.to_string(),
                    puzzle_id: puzzle_id.to_string(),
                    difficulty,
                    date,
                };

                let rating = Self::try_read(&row, "rating")?;

                Ok((review, rating))
            })
            .collect()
    }

    /// Get the number of cards in the database.
    pub fn get_card_count(&self) -> DbResult<i64> {
        // Get card and review count.
        const QUERY: &'static str = "
            SELECT
                COUNT(*) AS card_count
            FROM cards
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Self::try_read::<i64>(&row, "card_count")
            })
            .unwrap_or(Ok(0))
    }

    /// Get the number of reviews in the database.
    pub fn get_review_count(&self) -> DbResult<i64> {
        // Get card and review count.
        const QUERY: &'static str = "
            SELECT
                COALESCE(SUM(review_count), 0) AS review_count
            FROM cards
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .next()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Self::try_read::<i64>(&row, "review_count")
            })
            .unwrap_or(Ok(0))
    }

    /// Get the number of reviews due before a certain time.
    pub fn reviews_due_by(&self, date: DateTime<FixedOffset>) -> DbResult<i64> {
        const QUERY: &'static str = "
            SELECT count(*) as card_count
            FROM cards
            WHERE datetime(due) < datetime(?)
            ORDER BY datetime(due) ASC
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, date.to_rfc3339().as_str())).map_err(Self::convert_error)?
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Self::try_read(&row, "card_count")
            })
            .next()
            .unwrap_or(Ok(0))
    }
}
