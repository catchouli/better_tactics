pub mod db;

use std::error::Error;
use std::fs::File;
use std::io::Write;

use crate::db::PuzzleDatabase;

const MIN_RATING: i32 = 1000;
const MAX_RATING: i32 = 1100;
const MAX_PUZZLES: i32 = 50;

const LICHESS_DB_NAME: &'static str = "lichess_db_puzzle.csv.zst";
const SQLITE_DB_NAME: &'static str = "puzzles.sqlite";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    log::info!("Better tactics starting!");

    // Open puzzle database.
    let mut puzzle_db = PuzzleDatabase::open(SQLITE_DB_NAME)?;

    // If no puzzles are loaded into the db yet, initialise it from a copy of the lichess database.
    let puzzle_count = puzzle_db.count()?;
    if puzzle_count == 0 {
        log::info!("Puzzle database empty, initialising from {LICHESS_DB_NAME}");
        puzzle_db.init_database(LICHESS_DB_NAME)?;
        log::info!("Done initialising puzzle database");
    }
    else {
        log::info!("Loaded puzzle database with {puzzle_count} puzzles");
    }

    // Get puzzles in rating range.
    log::info!("Getting up to {MAX_PUZZLES} puzzles in rating range {MIN_RATING} to {MAX_RATING}");
    let puzzles = puzzle_db.get_puzzles_by_rating(MIN_RATING, MAX_RATING, MAX_PUZZLES)?;

    {
        let output_path = format!("Puzzles_x{}_from_{}_to_{}.pgn", MAX_PUZZLES, MIN_RATING, MAX_RATING);
        log::info!("Writing {} puzzles to {}", puzzles.len(), output_path);
        let mut output_file = File::create(&output_path)?;
        for puzzle in puzzles {
            let pgn = puzzle.to_pgn();
            write!(output_file, "{}\n\n", pgn?)?;
        }
    }

    Ok(())
}
