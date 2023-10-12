use std::fmt::Display;
use askama::Template;
use serde::Deserialize;
use warp::reply::Reply;

use crate::config::SrsConfig;
use crate::rating::{GameResult, Rating};
use crate::route::BaseTemplateData;
use crate::db::Puzzle;
use crate::services::ServiceError;
use crate::services::tactics_service::TacticsService;
use crate::services::user_service::{UserService, Stats};
use crate::srs::{Difficulty, Card};
use crate::time::{LocalTimeProvider, TimeProvider};
use crate::util;

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

    /// Get the puzzle id.
    fn get_puzzle_id(&self) -> Option<&str> {
        self.puzzle.as_ref().map(|p| p.puzzle_id.as_str())
    }
}

/// The POST request for reviewing a card.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewRequest {
    pub id: String,
    pub difficulty: i64,
}

/// GET /tactics/by_id/{puzzle_id}
pub async fn specific_puzzle(srs_config: SrsConfig, user_service: UserService,
    tactics_service: TacticsService, puzzle_id: String) -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get the user's rating and stats.
    let user_rating = user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get the specified puzzle and card.
    let (puzzle, card) = tactics_service.get_puzzle_by_id(&puzzle_id).await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Specific,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
        srs_config,
    })
}

/// GET /tactics/new
pub async fn random_puzzle(srs_config: SrsConfig, user_service: UserService,
    tactics_service: TacticsService) -> Result<PuzzleTemplate, warp::Rejection>
{
    // Get the user's rating and stats.
    let user_rating = user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get random puzzle in rating range.
    // TODO: note that this can actually return a card that isn't new (it's unlikely but in some
    // rating ranges maybe it's more likely?). We should filter by it, or try again.
    let (min_rating, max_rating) = tactics_service.get_rating_range(&user_rating).await?;
    let (puzzle, card) = tactics_service.get_random_puzzle(min_rating, max_rating).await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Random,
        puzzle,
        card,
        min_rating,
        max_rating,
        srs_config,
    })
}

/// GET /tactics
pub async fn next_review(srs_config: SrsConfig, user_service: UserService, tactics_service: TacticsService)
    -> Result<impl warp::Reply, warp::Rejection>
{
    // Get the user's rating and stats.
    let user_rating = user_service.get_user_rating(UserService::local_user_id()).await?;
    let stats = user_service.get_user_stats(UserService::local_user_id()).await?;

    // Get the next due puzzle and card.
    let (puzzle, card)  = tactics_service.get_next_review().await?;

    Ok(PuzzleTemplate {
        base: Default::default(),
        user_rating,
        stats,
        mode: PuzzleMode::Review,
        puzzle,
        card,
        min_rating: 0,
        max_rating: 0,
        srs_config,
    }.into_response())
}

/// POST /review
pub async fn review_puzzle(srs_config: SrsConfig, user_service: UserService,
    tactics_service: TacticsService, request: ReviewRequest) -> Result<impl warp::Reply, warp::Rejection>
{
    let user_id = UserService::local_user_id();

    let difficulty = Difficulty::from_i64(request.difficulty)
        .map_err(|_| ServiceError::InvalidParameter("difficulty".to_string()))?;

    // Get the puzzle for this puzzle id.
    let (puzzle, card) = tactics_service.get_puzzle_by_id(&request.id).await?;
    let puzzle = puzzle.ok_or(ServiceError::from(format!("No such puzzle {}", request.id)))?;
    let card = card.unwrap_or(Card::new::<LocalTimeProvider>(&request.id, srs_config));

    // Review the card.
    tactics_service.apply_review(user_id, card, difficulty).await?;

    // Update the user's rating.
    user_service.update_rating(user_id, difficulty, GameResult {
        rating: puzzle.rating,
        deviation: puzzle.rating_deviation,
        score: difficulty.score(),
    }).await?;

    Ok(warp::reply())
}
