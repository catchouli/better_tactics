use axum::extract::{State, Json};
use serde::Deserialize;

use crate::api::ApiError;
use crate::rating::GameResult;
use crate::app::AppState;
use crate::services::ServiceError;
use crate::services::user_service::UserService;
use crate::srs::{Difficulty, Card};
use crate::time::LocalTimeProvider;

/// Request JSON for /api/tactics/review.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewRequest {
    pub id: String,
    pub difficulty: i64,
}

/// POST /api/tactics/review.
/// TODO: why is this slow?
pub async fn review(
    State(state): State<AppState>,
    Json(request): Json<ReviewRequest>,
) -> Result<String, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    // TODO: change error type.
    let difficulty = Difficulty::from_i64(request.difficulty)
        .map_err(|_| ApiError::InvalidParameter("difficulty".into()))?;

    // Get the puzzle for this puzzle id.
    let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&request.id).await?;
    let puzzle = puzzle.ok_or(ServiceError::from(format!("No such puzzle {}", request.id)))?;
    let card = card.unwrap_or(Card::new::<LocalTimeProvider>(&request.id, state.app_config.srs));

    // Update the user's rating.
    let new_rating = state.user_service.update_rating(user_id, difficulty, GameResult {
        rating: puzzle.rating,
        deviation: puzzle.rating_deviation,
        score: difficulty.score(),
    }).await?;

    // Review the card.
    state.tactics_service.apply_review(user_id, new_rating, card, difficulty).await?;

    // TODO: seems unnecessary.
    Ok("".into())
}
