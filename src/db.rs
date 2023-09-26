use std::error::Error;
use std::fs::File;
use sqlite::Connection;
use owlchess::board::Board;
use owlchess::moves::Style;
use owlchess::chain::{MoveChain, NumberPolicy, GameStatusPolicy};

pub struct PuzzleDatabase {
    conn: Connection,
}

#[derive(Debug)]
pub struct Puzzle {
    pub puzzle_id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i32,
}

impl PuzzleDatabase {
    pub fn open(path: &str) -> Result<Self, Box<dyn Error>> {
        let conn = Self::init_schema(sqlite::open(path)?)?;

        Ok(Self {
            conn
        })
    }

    fn init_schema(conn: Connection) -> Result<Connection, Box<dyn Error>> {
        log::info!("Initialising db schema");
        const QUERY: &'static str = "
            CREATE TABLE IF NOT EXISTS puzzles (
                puzzle_id TEXT NOT NULL,
                fen TEXT NOT NULL,
                moves TEXT NOT NULL,
                rating INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS id_rating ON puzzles(puzzle_id);
            CREATE INDEX IF NOT EXISTS rating_index ON puzzles(rating);
        ";
        conn.execute(QUERY)?;
        Ok(conn)
    }

    pub fn count(&self) -> Result<usize, Box<dyn Error>> {
        const QUERY: &'static str = "
            SELECT COUNT(puzzle_id) FROM puzzles;
        ";

        if let Some(result) = self.conn.prepare(QUERY)?.into_iter().next() {
            let count = result?.read::<i64, _>(0);
            Ok(count as usize)
        }
        else {
            Ok(0)
        }
    }

    pub fn init_database(&mut self, lichess_db_raw: File) -> Result<(), Box<dyn Error>> {
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
                let rating = record[3].parse::<i32>()?;

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

    pub fn add_puzzle(&mut self, puzzle: &Puzzle) -> Result<(), Box<dyn Error>> {
        const QUERY: &'static str = "
            INSERT INTO puzzles (puzzle_id, fen, moves, rating) VALUES (?, ?, ?, ?)
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

    pub fn add_puzzles(&mut self, puzzles: &Vec<Puzzle>) -> Result<(), Box<dyn Error>> {
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

    pub fn get_puzzles_by_rating(&self, min_rating: i32, max_rating: i32, max_puzzles: i32)
        -> Result<Vec<Puzzle>, Box<dyn Error>>
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
                result.map(|row| {
                    let id = row.read::<&str, _>("puzzle_id");
                    let fen = row.read::<&str, _>("fen");
                    let moves = row.read::<&str, _>("moves");
                    let rating = row.read::<i64, _>("rating");
                    Puzzle::new(id, fen, moves, rating as i32)
                })
            })
            .into_iter()
            .collect();
        log::info!("Done");

        Ok(puzzles?)
    }
}

impl Puzzle {
    pub fn new(id: &str, fen: &str, moves: &str, rating: i32) -> Self {
        Puzzle {
            puzzle_id: id.to_string(),
            fen: fen.to_string(),
            moves: moves.to_string(),
            rating,
        }
    }

    // Convert the embedded fen and moves from UCI format to a PGN.
    pub fn to_pgn(&self) -> Result<String, Box<dyn Error>> {
        let pgn = Self::uci_to_pgn(&self.fen, &self.moves);
        Ok(format!("[Event \"Lichess puzzle {} (rating {})\"]
[Site \"?\"]
[Date \"????.??.??\"]
[Round \"?\"]
[White \"?\"]
[Black \"?\"]
[Result \"1-0\"]
[SetUp \"1\"]
[FEN \"{}\"]

{}", self.puzzle_id, self.rating, self.fen, pgn?))
    }

    fn uci_to_pgn(fen: &str, moves: &str) -> Result<String, Box<dyn Error>> {
        let board = Board::from_fen(fen)?;
        let movechain = MoveChain::from_uci_list(board, moves)?;
        Ok(movechain.styled(NumberPolicy::FromBoard, Style::San, GameStatusPolicy::Show).to_string())
    }
}
