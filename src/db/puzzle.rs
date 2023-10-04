use crate::db::{PuzzleDatabase, DbResult};

/// A puzzle record from the db.
#[derive(Debug, Clone)]
pub struct Puzzle {
    pub puzzle_id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i64,
}

/// Puzzle related database implementations.
impl PuzzleDatabase {
    /// Get the rating of the highest rated puzzle in the database.
    pub fn get_max_puzzle_rating(&self) -> DbResult<i64> {
        const QUERY: &'static str = "
            SELECT rating
            FROM puzzles
            ORDER BY rating DESC
            LIMIT 1
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Self::try_read(&row, "rating")
            })
            .next()
            .unwrap_or(Ok(0))
    }

    /// Get the number of puzzles in the database.
    pub fn get_puzzle_count(&self) -> DbResult<usize> {
        const QUERY: &'static str = "
            SELECT COUNT(puzzle_id) as puzzle_count FROM puzzles;
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;

                Ok(Self::try_read::<i64>(&row, "puzzle_count")? as usize)
            })
            .next()
            .unwrap_or(Ok(0))
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

        let finished_query = format!(
            "INSERT OR REPLACE INTO puzzles (puzzle_id, fen, moves, rating) VALUES {}",
            rows);

        self.conn.execute(finished_query)
            .map_err(Self::convert_error)
    }

    /// Get a puzzle by ID.
    pub fn get_puzzle_by_id(&self, puzzle_id: &str) -> DbResult<Option<Puzzle>>
    {
        const QUERY: &'static str = "
            SELECT fen, moves, rating
            FROM puzzles
            WHERE puzzle_id = ?
        ";

        let puzzle_id = puzzle_id.to_string();

        log::info!("Getting puzzle {puzzle_id}");

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, puzzle_id.as_str())).map_err(Self::convert_error)?
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Ok(Puzzle::new(
                    puzzle_id.as_str(), 
                    Self::try_read(&row, "fen")?,
                    Self::try_read(&row, "moves")?,
                    Self::try_read(&row, "rating")?,
                ))
            })
            .next()
            .transpose()
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
        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, min_rating as i64)).map_err(Self::convert_error)?
            .bind((2, max_rating as i64)).map_err(Self::convert_error)?
            .bind((3, max_puzzles as i64)).map_err(Self::convert_error)?
            .map(|result| {
                let row = result.map_err(Self::convert_error)?;
                Ok(Puzzle::new(
                    Self::try_read(&row, "puzzle_id")?, 
                    Self::try_read(&row, "fen")?,
                    Self::try_read(&row, "moves")?,
                    Self::try_read(&row, "rating")?,
                ))
            })
            .collect::<DbResult<Vec<Puzzle>>>()
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
}
