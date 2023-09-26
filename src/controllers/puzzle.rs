use std::sync::Arc;
use askama::Template;
use tokio::sync::Mutex;

use crate::db::{Puzzle, PuzzleDatabase};

/// The template for displaying puzzles.
#[derive(Template)]
#[template(path = "puzzle.html")]
pub struct PuzzleTemplate {
    puzzle: Puzzle,
}

pub async fn random_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>) -> Result<PuzzleTemplate, warp::Rejection> {
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get a random puzzle.
    // TODO: the unwrap is unsafe.
    let random_puzzle = puzzle_db.get_puzzles_by_rating(crate::MIN_RATING, crate::MAX_RATING, 1)
        .map(|vec| vec.into_iter().nth(0).unwrap() )
        .map_err(|_| warp::reject::not_found())?;

    Ok(PuzzleTemplate { puzzle: random_puzzle })
}
