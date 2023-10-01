use std::fmt::Display;
use std::sync::Arc;
use askama::Template;
use chrono::{Utc, Local};
use tokio::sync::Mutex;
use serde::Deserialize;
use warp::reply::Reply;

use crate::rating::GameResult;
use crate::route::{InternalError, InvalidParameter, NotFound};
use crate::util;
use crate::db::{Puzzle, PuzzleDatabase, Stats, Review, User};
use crate::srs::{Difficulty, Card};

/// The rating variation for puzzles, in percent.
const PUZZLE_RATING_VARIATION: f64 = 1.05;

/// The rating deviation for puzzles. This represents the certainty of the 'rating' for each puzzle
/// in our database, and a higher number should cause the rating algorithm to make less drastic
/// changes each time a puzzle is solved. In theory we could get this from the lichess puzzle db,
/// but I didn't import it to begin with, and doing it now would break any existing databases since
/// we don't have a way to migrate them.
const PUZZLE_RATING_DEVIATION: i64 = 50;

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
    user: User,
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
    let user_id = PuzzleDatabase::local_user_id();

    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the user's stats.
    let review_cutoff = Card::due_time().map_err(InternalError::from)?;
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).map_err(InternalError::from)?;

    // Get the user.
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

    // Get the puzzle.
    let puzzle = puzzle_db.get_puzzle_by_id(&puzzle_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| NotFound::new(&format!("Puzzle {puzzle_id}")))?;

    // Get the card for this puzzle (or a new empty card if it doesn't already exist).
    let card = puzzle_db.get_card_by_id(&puzzle.puzzle_id)
        .map_err(InternalError::from)?
        .unwrap_or(Card::new(&puzzle_id));

    Ok(PuzzleTemplate {
        mode: PuzzleMode::Specific,
        puzzle,
        stats,
        card,
        user,
        min_rating: 0,
        max_rating: 0,
    })
}

pub async fn random_puzzle(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<PuzzleTemplate, warp::Rejection>
{
    let user_id = PuzzleDatabase::local_user_id();

    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the user.
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

    // Get the user's stats.
    let review_cutoff = Card::due_time().map_err(InternalError::from)?;
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).map_err(InternalError::from)?;

    // The min and max rating for puzzles, deviating from the user's rating by up to
    // PUZZLE_RATING_VARIATION.
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
        user,
        min_rating,
        max_rating,
    })
}

pub async fn next_review(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<impl warp::Reply, warp::Rejection>
{
    let user_id = PuzzleDatabase::local_user_id();

    // Get connection to puzzle db.
    let puzzle_db = puzzle_db.lock().await;

    // Get the user's stats.
    let review_cutoff = Card::due_time().map_err(InternalError::from)?;
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff).map_err(InternalError::from)?;

    // Get the user.
    let user = puzzle_db.get_user_by_id(user_id)
        .map_err(InternalError::from)?
        .ok_or_else(|| InternalError::new(format!("Failed to get local user")))?;

    // Get the user's next due review. The logic for this is a little complicated as we want to
    // show cards that are due any time today (before the review cutoff) so the user can do their
    // reviews all at once rather than them trickling in throughout the day. On the other hand, we
    // don't want cards that are still in learning, with potentially low intervals, to show up
    // before they're due unless there's absolutely no other cards left, because otherwise they'll
    // show up repeatedly in front of other cards that are due later today.
    let time_now = Local::now().fixed_offset();
    let new_review_due_now = puzzle_db.get_next_review_due(time_now, None)
        .map_err(InternalError::from)?;

    let max_learning_interval = crate::srs::INITIAL_INTERVALS.last().map(|d| *d);
    let review_cutoff_today = Card::due_time().map_err(InternalError::from)?;
    let next_non_learning_review_due_today =
        puzzle_db.get_next_review_due(review_cutoff_today, max_learning_interval)
            .map_err(InternalError::from)?;

    let next_review = new_review_due_now.or(next_non_learning_review_due_today);

    if let Some((card, puzzle)) = next_review {
        Ok(PuzzleTemplate {
            mode: PuzzleMode::Review,
            puzzle,
            stats,
            card,
            user,
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
            .map_err(|_| InvalidParameter::new(&request.id))?
            .ok_or_else(|| InternalError::new(format!("No such puzzle {}", card.id)))?;

        // Apply the review to the card.
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

        // Update the user's rating every time a puzzle is solved, old puzzles don't give much
        // rating anymore once your rating deviation is low enough.
        let old_rating = user.rating;

        user.rating.update(vec![GameResult {
            rating: puzzle.rating,
            deviation: PUZZLE_RATING_DEVIATION,
            score: difficulty.score(),
        }]);

        // The downside is that sometimes 'Good' ratings for low rated puzzles actually lower the
        // user's rating, due to the way we fudge the score for them. (They're scored somewhere
        // between 'Easy', which is a win at 1.0, and 'Hard', which is a draw at 0.0). As a bit of
        // an arbitrary fix, we just prevent 'Good' ratings from lowering the user's rating.
        if difficulty != Difficulty::Good || user.rating.rating > old_rating.rating {
            log::info!("Updating user's rating from {} to {}", old_rating.rating, user.rating.rating);

            puzzle_db.update_user(&user)
                .map_err(InternalError::from)?;
        }
    }

    Ok(warp::reply())
}
