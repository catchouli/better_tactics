use std::error::Error;
use std::fs::File;
use std::io::Write;

use crate::db::PuzzleDatabase;
use crate::{MIN_RATING, MAX_RATING, MAX_PUZZLES};

/// The original pgn generator, in case it's still needed at some point.
pub fn output_random_pgn(puzzle_db: &PuzzleDatabase) -> Result<(), Box<dyn Error>> {
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
