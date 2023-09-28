use std::fmt::Display;
use std::sync::Arc;
use askama::Template;
use chrono::Utc;
use tokio::sync::Mutex;
use serde::Deserialize;

use crate::db::{Puzzle, PuzzleDatabase};
use crate::srs::{Difficulty, Card};

/// The puzzle mode.
#[derive(Debug, PartialEq, Eq)]
pub enum PuzzleMode {
    /// We're showing a review.
    Review,

    /// We're showing a random new puzzle.
    Random,

    /// We're showing a specifically requested puzzle.
    Specific,
}

impl Display for PuzzleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PuzzleMode::Review => write!(f, "Review"),
            PuzzleMode::Random => write!(f, "Random"),
            PuzzleMode::Specific => write!(f, "Specific"),
        }
    }
}

/// The template for displaying puzzles.
#[derive(Template)]
#[template(path = "puzzle.html")]
pub struct PuzzleTemplate {
    mode: PuzzleMode,
    puzzle: Puzzle,
    card: Card,
    min_rating: i64,
    max_rating: i64,
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

    // Get the puzzle.
    let puzzle = puzzle_db.get_puzzle_by_id(&puzzle_id)
        .map_err(|_| warp::reject::not_found())?;

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(|_| warp::reject::not_found())?;

    Ok(PuzzleTemplate {
        mode: PuzzleMode::Specific,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
    })
}

pub async fn random_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the min and max ratings for puzzles.
    // TODO: base this on the user's rating.
    let min_rating = crate::MIN_RATING;
    let max_rating = crate::MAX_RATING;

    // Get a random puzzle.
    // TODO: unsafe unwrap.
    let puzzle = puzzle_db.get_puzzles_by_rating(min_rating, max_rating, 1)
        .map(|vec| vec.into_iter().nth(0))
        .map_err(|_| warp::reject::not_found())?
        .unwrap();

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(|_| warp::reject::not_found())?;

    Ok(PuzzleTemplate {
        mode: PuzzleMode::Random,
        puzzle,
        card,
        min_rating,
        max_rating,
    })
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
        puzzle_db.update_or_create_card(&card).unwrap();
    }

    Ok(warp::reply())
}
