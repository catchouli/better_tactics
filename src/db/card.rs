
use chrono::{DateTime, FixedOffset, Duration};
use futures::{TryStreamExt, StreamExt};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::srs::{Card, Difficulty};
use crate::db::{PuzzleDatabase, DbResult, Puzzle, ErrorDetails};

use super::DatabaseError;

/// A review record from the db.
#[derive(Debug, Clone)]
pub struct Review {
    pub user_id: String,
    pub puzzle_id: String,
    pub difficulty: Difficulty,
    pub date: DateTime<FixedOffset>,
    pub user_rating: Option<i64>,
}

/// A bucket of review scores for puzzles in the given review range with the given score.
#[derive(Debug)]
pub struct ReviewScoreBucket {
    pub puzzle_rating_min: i64,
    pub puzzle_rating_max: i64,
    pub difficulty: Difficulty,
    pub review_count: i64,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Review
{
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            user_id: row.try_get("user_id")?,
            puzzle_id: row.try_get("user_id")?,
            difficulty: Difficulty::from_i64(row.try_get("difficulty")?)
                .map_err(|e| sqlx::Error::ColumnDecode {
                    index: "Difficulty".to_string(),
                    source: e.to_string().into(),
                })?,
            date: DateTime::parse_from_rfc3339(row.try_get("date")?)
                .map_err(|e| sqlx::Error::ColumnDecode {
                    index: "date".to_string(),
                    source: e.to_string().into(),
                })?,
            user_rating: row.try_get("user_rating").ok(),
        })
    }
}

impl PuzzleDatabase {
    /// Build a card from a result row. The reason we have it defined here instead of as a FromRow
    /// instance is because we need access to self.srs_config.
    fn card_from_row<'r>(&self, row: &'r SqliteRow) -> Result<Card, sqlx::Error> {
        Ok(Card {
            id: row.try_get("puzzle_id")?,
            interval: Duration::seconds(row.try_get("interval")?),
            review_count: row.try_get("review_count")?,
            ease: row.try_get("ease")?,
            learning_stage: row.try_get("learning_stage")?,
            due: DateTime::parse_from_rfc3339(row.try_get("due")?)
                .map_err(|e| sqlx::Error::ColumnDecode {
                    index: "due".to_string(),
                    source: e.to_string().into(),
                })?,
            srs_config: self.srs_config,
        })
    }

    /// Get the next due review. min_interval allows us to filter out cards with short intervals
    /// (e.g. because they're still in learning), because otherwise they'll show up, possibly
    /// repeatedly if learning or relearning, before other cards that are due later today.
    pub async fn get_next_review_due(&self, time: DateTime<FixedOffset>, min_interval: Option<Duration>)
        -> DbResult<Option<(Card, Puzzle)>>
    {
        let min_interval_seconds = min_interval.map(|i| i.num_seconds()).unwrap_or(0);

        // TODO: if the puzzles for a card get deleted it will cause a weird disconnect between the
        // user's due count and the cards they have due. For now we filter out cards with a NULL
        // puzzle_id to stop them showing up in reviews but they'll still have more than 0 reviews
        // forever which is weird.
        let query = sqlx::query("
            SELECT * FROM cards
            LEFT JOIN puzzles
                ON cards.puzzle_id = puzzles.puzzle_id
            WHERE datetime(due) <= datetime(?)
            AND interval >= ?
            AND puzzles.puzzle_id NOT NULL
            ORDER BY datetime(due) ASC
            LIMIT 1
        ");

        query
            .bind(time.to_rfc3339().as_str())
            .bind(min_interval_seconds)
            .fetch_optional(&self.pool)
            .await?
            .map(|row: SqliteRow| {
                let card: Card = self.card_from_row(&row)?;
                let puzzle: Puzzle = sqlx::FromRow::from_row(&row)?;
                Ok((card, puzzle))
            })
            .transpose()
    }

    /// Get a single card by ID.
    pub async fn get_card_by_id(&self, puzzle_id: &str) -> DbResult<Option<Card>> {
        log::info!("Getting card for puzzle {puzzle_id}");

        let query = sqlx::query("
            SELECT *
            FROM cards
            WHERE puzzle_id = ?
        ");

        Ok(query
            .bind(puzzle_id)
            .map(|row| self.card_from_row(&row))
            .fetch_optional(&self.pool)
            .await?
            .transpose()?)
    }

    /// Update (or create) a card by ID.
    pub async fn update_or_create_card(&mut self, card: &Card) -> DbResult<()> {
        log::info!("Updating card for puzzle {}: {card:?}", card.id);

        let query = sqlx::query("
            INSERT OR REPLACE INTO cards (puzzle_id, due, interval, review_count, ease, learning_stage)
            VALUES (?, ?, ?, ?, ?, ?)
        ");

        query
            .bind(&card.id)
            .bind(card.due.to_rfc3339())
            .bind(card.interval.num_seconds())
            .bind(card.review_count)
            .bind(card.ease)
            .bind(card.learning_stage)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add a review record for a user.
    pub async fn add_review_for_user(&mut self, review: Review) -> DbResult<()>
    {
        let query = sqlx::query("
            INSERT INTO reviews (user_id, puzzle_id, difficulty, date, user_rating)
            VALUES (?, ?, ?, ?, ?)
        ");

        query
            .bind(&review.user_id)
            .bind(&review.puzzle_id)
            .bind(review.difficulty.to_i64())
            .bind(review.date.to_rfc3339())
            .bind(review.user_rating)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a rating history for a user. 
    pub async fn get_user_rating_history(&self, user_id: &str)
        -> DbResult<Vec<(DateTime<FixedOffset>, i64)>>
    {
        let query = sqlx::query(r#"
            SELECT * FROM (
                SELECT reviews.date, max(user_rating) as max_rating
                FROM reviews
                WHERE user_id = ?
                AND user_rating IS NOT NULL
                -- Group by day and then hour
                GROUP BY date(reviews.date), strftime('%H', reviews.date)
                -- Add the user's current rating to the end
                UNION SELECT strftime("%Y-%m-%dT%H:%M:%SZ", datetime("now")),
                    rating FROM users WHERE id = ?
            )
            ORDER BY datetime(date)
        "#);

        Ok(query
            .bind(user_id)
            .bind(user_id)
            .fetch(&self.pool)
            .map(|row: Result<SqliteRow, _>| {
                let row = row?;
                let date = DateTime::parse_from_rfc3339(row.try_get("date")?)?;
                let rating: i64 = row.try_get("max_rating")?;
                Ok((date, rating)) as Result<_, DatabaseError>
            })
            .try_collect()
            .await?)
    }

    /// Get the number of cards in the database.
    pub async fn get_card_count(&self) -> DbResult<i64> {
        // Get card and review count.
        let query = sqlx::query("
            SELECT COUNT(*) as card_count
            FROM cards
        ");

        Ok(query
            .fetch_optional(&self.pool)
            .await?
            .map(|row| row.try_get("card_count"))
            .unwrap_or(Ok(0))?)
    }

    /// Get the number of reviews in the database.
    pub async fn get_review_count(&self) -> DbResult<i64> {
        // Get card and review count.
        let query = sqlx::query("
            SELECT
                COALESCE(SUM(review_count), 0) AS review_count
            FROM cards
        ");

        Ok(query
            .fetch_optional(&self.pool)
            .await?
            .map(|row| row.try_get("review_count"))
            .unwrap_or(Ok(0))?)
    }

    /// Get the number of reviews due by `time`, including reviewing ahead until `day_end`, but
    /// only if the card is out of learning.
    pub async fn reviews_due_by(&self, time: DateTime<FixedOffset>, day_end: DateTime<FixedOffset>)
        -> DbResult<i64>
    {
        let query = sqlx::query("
            SELECT count(*) as card_count
            FROM cards
            WHERE (datetime(due) <= datetime(?)
                    AND cards.interval >= ?)
            OR datetime(due) <= datetime(?)
        ");

        let max_learning_interval = crate::srs::INITIAL_INTERVALS.last().map(|d| *d)
            .map(|interval| interval.num_seconds())
            .unwrap_or(0);

        Ok(query
            .bind(day_end.to_rfc3339())
            .bind(max_learning_interval)
            .bind(time.to_rfc3339())
            .fetch_optional(&self.pool)
            .await?
            .map(|row| row.try_get("card_count"))
            .unwrap_or(Ok(0))?)
    }

    /// Get the review score history for a user, in buckets of `rating_bucket_span` rating span.
    /// e.g. rating_bucket_span = 50 means you'll get buckets every 50 puzzle rating, so 450-500,
    /// 500-550, etc.
    pub async fn get_review_score_history(&self, user_id: &str, rating_bucket_span: i64)
        -> DbResult<Vec<ReviewScoreBucket>>
    {
        let query = sqlx::query("
            SELECT reviews.difficulty,
                puzzles.rating - puzzles.rating % ? AS min_rating,
                count(reviews.difficulty) AS review_count
            FROM reviews
            JOIN puzzles ON reviews.puzzle_id = puzzles.puzzle_id
            WHERE reviews.user_id = ?
            GROUP BY min_rating, difficulty
            ORDER BY min_rating, difficulty
        ");

        Ok(query
            .bind(rating_bucket_span)
            .bind(user_id)
            .fetch(&self.pool)
            .map(|row| {
                let row = row?;
                let min_rating = row.try_get("min_rating")?;
                Ok(ReviewScoreBucket {
                    puzzle_rating_min: min_rating,
                    puzzle_rating_max: min_rating + rating_bucket_span,
                    difficulty: Difficulty::from_i64(row.try_get("difficulty")?)
                        .map_err(|e| DatabaseError::ParsingError(ErrorDetails {
                            backend: "srs".to_string(),
                            description: e.to_string(),
                            source: Some(e),
                        }))?,
                    review_count: row.try_get("review_count")?,
                }) as DbResult<ReviewScoreBucket>
            })
            .try_collect::<Vec<ReviewScoreBucket>>()
            .await?)
    }

    /// Get the review forecast for a user.
    pub async fn get_review_forecast(&self, day_end: DateTime<FixedOffset>, max_days: i64)
        -> DbResult<Vec<(i64, i64)>>
    {
        let query = sqlx::query("
            -- This bit of voodoo calculates the day the card is due as a fractional value,
            -- (e.g. 0.5 is today, but 1.0 is at the day start time tommorow morning),
            -- then floors it to get an integer value and groups by it to get the number of
            -- cards due on each day.
            SELECT CAST((JULIANDAY(due) - JULIANDAY(?)) as integer) as day_due,
                count(ROWID) as reviews_due
            FROM cards
            WHERE day_due < ?
            GROUP BY day_due
        ");

        query
            .bind(day_end.to_rfc3339())
            .bind(max_days)
            .fetch(&self.pool)
            .map(|row| {
                let row = row?;
                let day_due = row.try_get::<i64, _>("day_due")?;
                let reviews_due = row.try_get::<i64, _>("reviews_due")?;
                Ok((day_due, reviews_due)) as DbResult<(i64, i64)>
            })
        .try_collect::<Vec<(i64, i64)>>()
        .await
    }
}
