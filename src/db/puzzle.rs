use std::fs::File;

use std::error::Error;
use owlchess::board::Board;
use owlchess::moves::Style;
use owlchess::chain::{MoveChain, NumberPolicy, GameStatusPolicy};

use crate::db::{DatabaseError, ErrorDetails, PuzzleDatabase, DbResult};

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
                let record = result.map_err(|e| DatabaseError::DataImportError(ErrorDetails {
                    backend: "csv".to_string(),
                    description: format!("CSV parse error when importing lichess puzzles database: {e}"),
                    source: Some(e.into()),
                }))?;

                if record.len() != EXPECTED_ROWS {
                    log::warn!("Skipping record with {} entries, expected at least {}", record.len(), EXPECTED_ROWS);
                    continue;
                }

                // Import puzzle.
                let puzzle_id = &record[0];
                let fen = &record[1];
                let moves = &record[2];
                let rating = Self::try_parse(&record[3])?;

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

    /// Add a single puzzle to the database.
    pub fn add_puzzle(&mut self, puzzle: &Puzzle) -> DbResult<()> {
        const QUERY: &'static str = "
            INSERT INTO puzzles (puzzle_id, fen, moves, rating)
            VALUES (?, ?, ?, ?)
        ";

        self.conn
            .prepare(QUERY).map_err(Self::convert_error)?
            .into_iter()
            .bind((1, puzzle.puzzle_id.as_str())).map_err(Self::convert_error)?
            .bind((2, puzzle.fen.as_str())).map_err(Self::convert_error)?
            .bind((3, puzzle.moves.as_str())).map_err(Self::convert_error)?
            .bind((4, puzzle.rating as i64)).map_err(Self::convert_error)?
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
