use std::fmt::Display;
use std::sync::Arc;
use askama::Template;
use chrono::Utc;
use tokio::sync::Mutex;
use serde::Deserialize;
use warp::reply::Reply;

use crate::route::{InvalidParameter, InternalError};
use crate::util;
use crate::db::{Puzzle, PuzzleDatabase, Stats};
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
    stats: Stats,
    min_rating: i64,
    max_rating: i64,
}

/// The tempalte for when there are no remaining reviews.
#[derive(Template)]
#[template(path = "reviews-done.html")]
pub struct ReviewsDone;

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

    // Get the user's stats.
    let stats = puzzle_db.get_local_user_stats()
        .map_err(|e| InternalError::new(e.to_string()))?;

    // Get the puzzle.
    let puzzle = puzzle_db.get_puzzle_by_id(&puzzle_id)
        .map_err(|e| InternalError::new(e.to_string()))?;

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(|e| InternalError::new(e.to_string()))?
        .unwrap_or(Card::new(&puzzle_id));

    Ok(PuzzleTemplate {
        mode: PuzzleMode::Specific,
        puzzle,
        stats,
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

    // Get the user's stats.
    let stats = puzzle_db.get_local_user_stats()
        .map_err(|e| InternalError::new(e.to_string()))?;

    // Get the min and max ratings for puzzles.
    // TODO: base this on the user's rating.
    let min_rating = crate::MIN_RATING;
    let max_rating = crate::MAX_RATING;

    // Get a random puzzle.
    // TODO: unsafe unwrap.
    let puzzle = puzzle_db.get_puzzles_by_rating(min_rating, max_rating, 1)
        .map(|vec| vec.into_iter().nth(0))
        .map_err(|e| InternalError::new(e.to_string()))?
        .unwrap();

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(|e| InternalError::new(e.to_string()))?
        .unwrap_or(Card::new(&puzzle.puzzle_id));

    Ok(PuzzleTemplate {
        mode: PuzzleMode::Random,
        puzzle,
        stats,
        card,
        min_rating,
        max_rating,
    })
}

pub async fn next_review(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<impl warp::Reply, warp::Rejection>
{
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the user's stats.
    let stats = puzzle_db.get_local_user_stats()
        .map_err(|e| InternalError::new(e.to_string()))?;

    // Get the user's next due review.
    let next_review = puzzle_db.get_next_review_due()
        .map_err(|e| InternalError::new(e.to_string()))?;

    if let Some((card, puzzle)) = next_review {
        Ok(PuzzleTemplate {
            mode: PuzzleMode::Review,
            puzzle,
            stats,
            card,
            min_rating: 0,
            max_rating: 0,
        }.into_response())
    }
    else {
        Ok(ReviewsDone {}.into_response())
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

    // Update the card.
    if let Ok(card) = puzzle_db.get_card_by_id(request.id.as_str()) {
        let mut card = card.unwrap_or(Card::new(&request.id));

        card.review(Utc::now().fixed_offset(), difficulty);

        puzzle_db.update_or_create_card(&card)
            .map_err(|e| {
                InternalError::new(format!("{}", e.to_string()))
            })?;
    }

    // Increase the user's rating by 1 every time they complete a review.
    // TODO: very temporary, we need to figure out a better scheme for increasing the user's
    // rating.
    let mut user = puzzle_db.get_user_by_id(PuzzleDatabase::local_user_id())
        .map_err(|_| InternalError::new(format!("3")))?
        .unwrap();

    user.rating += 1;
    puzzle_db.update_user(&user)
        .map_err(|_| InternalError::new(format!("4")))?;

    Ok(warp::reply())
}
