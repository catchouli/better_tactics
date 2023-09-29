use std::fmt::Display;
use std::sync::Arc;
use askama::Template;
use chrono::{Utc, Local, Duration};
use tokio::sync::Mutex;
use serde::Deserialize;
use warp::reply::Reply;

use crate::rating::GameResult;
use crate::route::{InternalError, InvalidParameter};
use crate::util;
use crate::db::{Puzzle, PuzzleDatabase, Stats, Review};
use crate::srs::{Difficulty, Card};

/// The rating variation for puzzles, in percent.
const PUZZLE_RATING_VARIATION: f64 = 1.05;

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
    pub difficulty: i64,
}

pub async fn specific_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>, puzzle_id: String)
    -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the user's stats.
    let stats = puzzle_db.get_user_stats(PuzzleDatabase::local_user_id())
        .map_err(InternalError::from)?;

    // Get the puzzle.
    let puzzle = puzzle_db.get_puzzle_by_id(&puzzle_id)
        .map_err(InternalError::from)?;

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(InternalError::from)?
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

    // Get the user.
    let user_id = PuzzleDatabase::local_user_id();
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

    // Get the user's stats.
    let stats = puzzle_db.get_user_stats(user_id)
        .map_err(InternalError::from)?;

    // The min and max rating for puzzles, based on the user's rating, plus or minus a few percent,
    // according to PUZZLE_RATING_VARIATION.
    let min_rating = (user.rating.rating as f64 / PUZZLE_RATING_VARIATION) as i64;
    let max_rating = (user.rating.rating as f64 * PUZZLE_RATING_VARIATION) as i64;

    // Get a random puzzle.
    let puzzle = puzzle_db.get_puzzles_by_rating(min_rating, max_rating, 1)
        .map(|vec| vec.into_iter().nth(0))
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Got no puzzles when requesting a random one")))?;

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(InternalError::from)?
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
    let stats = puzzle_db.get_user_stats(PuzzleDatabase::local_user_id())
        .map_err(InternalError::from)?;

    // Get the user's next due review.
    let next_review = puzzle_db.get_next_review_due()
        .map_err(InternalError::from)?;

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
    let user_id = PuzzleDatabase::local_user_id();
    let mut puzzle_db = puzzle_db.lock().await;

    let difficulty = Difficulty::from_i64(request.difficulty)
        .map_err(|_| InvalidParameter::new("difficulty"))?;

    // Update the card.
    if let Ok(card) = puzzle_db.get_card_by_id(request.id.as_str()) {
        // If there's no existing card for the puzzle, create a new one.
        let mut card = card.unwrap_or(Card::new(&request.id));

        let mut user = puzzle_db.get_user_by_id(user_id)
            .map_err(InternalError::from)?
            .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

        let puzzle = puzzle_db.get_puzzle_by_id(&card.id)
            .map_err(|_| InvalidParameter::new(&request.id))?;

        // Apply the review to the card.
        let old_interval = card.interval;
        card.review(Utc::now().fixed_offset(), difficulty);

        // Create a review record in the database.
        puzzle_db.add_review_for_user(Review {
            user_id: user_id.to_string(),
            puzzle_id: card.id.to_string(),
            difficulty,
            date: Local::now().fixed_offset(),
        }).map_err(InternalError::from)?;

        // Update (or create) the card in the database.
        puzzle_db.update_or_create_card(&card)
            .map_err(InternalError::from)?;

        // If the card left learning, (e.g. its interval went past 1 day for the first time),
        // update the user's rating. Note that we don't currently enforce the 'first time' part,
        // but it might be worth only awarding rating when the user solves a puzzle for the first
        // time. We'd have to store that though (or figure it out from the information we already
        // store).
        let one_day = Duration::days(1);
        //if difficulty == Difficulty::Again || (old_interval < one_day && card.interval > one_day) {
            log::info!("First time pass, updating user's rating");

            let old_rating = user.rating.rating;

            user.rating.update(vec![GameResult {
                rating: puzzle.rating,
                deviation: 1,
                score: match difficulty {
                    Difficulty::Again => 0.0,
                    Difficulty::Hard => 0.5,
                    Difficulty::Good => 1.0,
                    Difficulty::Easy => 1.0,
                }
            }]);

            log::info!("Updating user's rating from {} to {}", old_rating, user.rating.rating);

            puzzle_db.update_user(&user)
                .map_err(InternalError::from)?;
        //}
    }


    //user.rating.rating += 1;

    // Update the user's rating based on a single 'game' against the puzzle.

    Ok(warp::reply())
}
