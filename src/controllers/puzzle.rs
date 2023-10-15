use std::fmt::Display;
use askama::Template;
use axum::extract::{State, Path};

use crate::rating::Rating;
use crate::app::AppState;
use crate::db::Puzzle;
use crate::services::user_service::{UserService, Stats};
use crate::srs::{Difficulty, Card, SrsConfig};
use crate::time::{LocalTimeProvider, TimeProvider};
use crate::util;

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
    user_rating: Rating,
    stats: Stats,
    mode: PuzzleMode,
    puzzle: Option<Puzzle>,
    card: Option<Card>,
    min_rating: i64,
    max_rating: i64,
    srs_config: SrsConfig,
}

impl PuzzleTemplate {
    /// Get the next interval after a review with score `score` in human readable form.
    fn next_interval_human(&self, score: Difficulty) -> String {
        let interval = self.card.as_ref().unwrap_or(&Card::new::<LocalTimeProvider>("", self.srs_config))
            .next_interval(score);
        crate::util::review_duration_to_human(interval)
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
    // Get the user's rating and stats.
    let user_rating = state.user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = state.user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get the specified puzzle and card.
    let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&puzzle_id).await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Specific,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
        srs_config: state.app_config.srs,
    })
}

/// GET /tactics/new
pub async fn random_puzzle(State(state): State<AppState>)
    -> Result<PuzzleTemplate, ControllerError>
{
    // Get the user's rating and stats.
    let user_rating = state.user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = state.user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get random puzzle in rating range.
    // TODO: note that this can actually return a card that isn't new (it's unlikely but in some
    // rating ranges maybe it's more likely?). We should filter by it, or try again.
    let (min_rating, max_rating) = state.tactics_service.get_rating_range(&user_rating).await?;
    let (puzzle, card) = state.tactics_service.get_random_puzzle(min_rating, max_rating).await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Random,
        puzzle,
        card,
        min_rating,
        max_rating,
        srs_config: state.app_config.srs,
    })
}

/// GET /tactics
pub async fn next_review(State(state): State<AppState>)
    -> Result<PuzzleTemplate, ControllerError>
{
    // Get the user's rating and stats.
    let user_rating = state.user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = state.user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get the next due puzzle and card.
    let (puzzle, card) = state.tactics_service.get_next_review().await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Review,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
        srs_config: state.app_config.srs,
    })
}
