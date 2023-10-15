use std::fmt::Display;
use askama::Template;
use axum::extract::{State, Path};

use crate::app::AppState;
use crate::db::Puzzle;
use crate::services::user_service::UserService;
use crate::srs::{Difficulty, Card};
use crate::time::{LocalTimeProvider, TimeProvider};

use super::{BaseTemplateData, ControllerError};

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
    base: BaseTemplateData,
    mode: PuzzleMode,
    puzzle: Option<Puzzle>,
    card: Card,
    min_rating: i64,
    max_rating: i64,
}

impl PuzzleTemplate {
    /// Get the next interval after a review with score `score` in human readable form.
    fn next_interval_human(&self, score: Difficulty) -> String {
        crate::util::review_duration_to_human(self.card.next_interval(score))
    }

    /// Get a human readable time until due for a card.
    fn human_readable_due(card: &Card) -> String {
        let time_until_due = card.due - LocalTimeProvider::now();
        crate::util::review_duration_to_human(time_until_due)
    }

    /// Check whether a card is due now.
    fn card_due_now(card: &Card) -> bool {
        card.is_due::<LocalTimeProvider>()
    }

    /// Get the puzzle themes as a list.
    fn puzzle_themes_list(&self) -> Option<String> {
        self.puzzle.as_ref().map(|p| p.themes.join(", "))
    }
}

/// GET /tactics/by_id/{puzzle_id}
pub async fn specific_puzzle(
    State(state): State<AppState>,
    Path(puzzle_id): Path<String>,
) -> Result<PuzzleTemplate, ControllerError>
{
    // Get the specified puzzle and card.
    let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&puzzle_id).await?;

    let card = card.unwrap_or(Card::new::<LocalTimeProvider>("", state.app_config.srs));

    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Specific,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
    })
}

/// GET /tactics/new
pub async fn random_puzzle(State(state): State<AppState>)
    -> Result<PuzzleTemplate, ControllerError>
{
    // Get random puzzle in rating range.
    // TODO: note that this can actually return a card that isn't new (it's unlikely but in some
    // rating ranges maybe it's more likely?). We should filter by it, or try again.
    let user_rating = state.user_service.get_user_rating(UserService::local_user_id()).await?;
    let (min_rating, max_rating) = state.tactics_service.get_rating_range(&user_rating).await?;
    let (puzzle, card) = state.tactics_service.get_random_puzzle(min_rating, max_rating).await?;

    let card = card.unwrap_or(Card::new::<LocalTimeProvider>("", state.app_config.srs));

    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Random,
        puzzle,
        card,
        min_rating,
        max_rating,
    })
}

/// GET /tactics
pub async fn next_review(State(state): State<AppState>)
    -> Result<PuzzleTemplate, ControllerError>
{
    // Get the next due puzzle and card.
    let (puzzle, card) = state.tactics_service.get_next_review().await?;

    let card = card.unwrap_or(Card::new::<LocalTimeProvider>("", state.app_config.srs));

    Ok(PuzzleTemplate {
        base: Default::default(),
        mode: PuzzleMode::Review,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
    })
}
