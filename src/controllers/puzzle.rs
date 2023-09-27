use std::sync::Arc;
use askama::Template;
use tokio::sync::Mutex;

use crate::db::{Puzzle, PuzzleDatabase};
use crate::srs::{self, Difficulty};

/// The template for displaying puzzles.
#[derive(Template)]
#[template(path = "puzzle.html")]
pub struct PuzzleTemplate {
    puzzle: Puzzle,
}

pub async fn specific_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>, puzzle_id: String)
    -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get a random puzzle.
    let random_puzzle = puzzle_db.get_puzzle_by_id(&puzzle_id)
        .map_err(|_| warp::reject::not_found())?;

    Ok(PuzzleTemplate { puzzle: random_puzzle })
}

pub async fn random_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get a random puzzle.
    let random_puzzle = puzzle_db.get_puzzles_by_rating(crate::MIN_RATING, crate::MAX_RATING, 1)
        .map(|vec| vec.into_iter().nth(0) );

    if let Ok(Some(puzzle)) = random_puzzle {
        Ok(PuzzleTemplate { puzzle })
    }
    else {
        Err(warp::reject::not_found())
    }
}

pub async fn review_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>, puzzle_id: String, difficulty: Difficulty)
    -> Result<impl warp::Reply, warp::Rejection>
{
    Ok(warp::reply())
}
