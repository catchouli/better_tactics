use std::error::Error;
use std::fs::File;
use std::io::Write;

use chrono::Duration;

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

// Convert a review duration to a human readable string, or "now" if it's negative.
pub fn review_duration_to_human(duration: &Duration) -> String {
    if duration.num_seconds() <= 0 {
        "now".to_string()
    }
    else if duration.num_weeks() > 0 {
        let weeks = duration.num_weeks();
        let days = duration.num_days() - weeks * 7;

        format!("{}w {}d", weeks, days)
    }
    else if duration.num_days() > 0 {
        let days = duration.num_days();
        let hours = duration.num_hours() - days * 24;

        format!("{}d {}h", days, hours)
    }
    else if duration.num_hours() > 0 {
        let hours = duration.num_hours();
        let mins = duration.num_minutes() - hours * 60;

        format!("{}h {}m", hours, mins)
    }
    else if duration.num_minutes() > 0 {
        let mins = duration.num_minutes();

        format!("{}m", mins)
    }
    else {
        let secs = duration.num_seconds();

        format!("{}s", secs)
    }
}

// TODO: a hack to get the template to compile, but probably unnecessary.
pub fn review_duration_to_human_owned(duration: Duration) -> String {
    review_duration_to_human(&duration)
}
