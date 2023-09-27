use std::sync::Arc;
use askama::Template;
use chrono::Utc;
use tokio::sync::Mutex;
use serde::Deserialize;

use crate::db::{Puzzle, PuzzleDatabase};
use crate::srs::Difficulty;

/// The template for displaying puzzles.
#[derive(Template)]
#[template(path = "puzzle.html")]
pub struct PuzzleTemplate {
    puzzle: Puzzle,
}

/// The POST request for reviewing a card.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewRequest {
    pub id: String,
    pub difficulty: i32,
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

pub async fn review_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>, request: ReviewRequest)
    -> Result<impl warp::Reply, warp::Rejection>
{
    let mut puzzle_db = puzzle_db.lock().await;

    // TODO: unsafe panic
    let difficulty = match request.difficulty {
        0 => Difficulty::Again,
        1 => Difficulty::Hard,
        2 => Difficulty::Good,
        3 => Difficulty::Easy,
        _ => panic!("Invalid difficulty {}", request.difficulty)
    };

    if let Ok(mut card) = puzzle_db.get_card_by_id(request.id.as_str()) {
        // TODO: unsafe unwrap
        card.review(Utc::now().fixed_offset(), difficulty).unwrap();
        log::info!("Updating card: {card:?}");
        puzzle_db.update_card(&card).unwrap();
    }

    Ok(warp::reply())
}
