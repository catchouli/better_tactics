use axum::extract::{State, Json, Path};
use chrono::Local;
use serde::Deserialize;
use serde::ser::SerializeStruct;

use crate::api::ApiError;
use crate::db::Puzzle;
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

/// Response JSON for endpoints returning a puzzle and a card.
#[derive(Debug, serde::Serialize)]
pub struct CardResponse {
    puzzle: Puzzle,
    #[serde(serialize_with = "serialize_card")]
    card: Card,
    due_today: bool,
}

fn serialize_card<S: serde::Serializer>(card: &Card, serializer: S) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_struct("Card", 10)?;
    s.serialize_field("id", &card.id)?;
    s.serialize_field("due", &card.due.to_rfc3339())?;
    s.serialize_field("interval", &card.interval.num_milliseconds())?;
    s.serialize_field("review_count", &card.review_count)?;
    s.serialize_field("ease", &card.ease)?;
    s.serialize_field("learning_stage", &card.learning_stage)?;
    // Add the next intervals so the app can display them on the review buttons.
    s.serialize_field("next_interval_again", &card.next_interval(Difficulty::Again).num_milliseconds())?;
    s.serialize_field("next_interval_hard", &card.next_interval(Difficulty::Hard).num_milliseconds())?;
    s.serialize_field("next_interval_good", &card.next_interval(Difficulty::Good).num_milliseconds())?;
    s.serialize_field("next_interval_easy", &card.next_interval(Difficulty::Easy).num_milliseconds())?;
    s.end()
}

/// POST /api/tactics/review.
pub async fn review(
    State(state): State<AppState>,
    Json(request): Json<ReviewRequest>,
) -> Result<(), ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let difficulty = Difficulty::from_i64(request.difficulty)
        .map_err(|_| ApiError::InvalidParameter("difficulty".into()))?;

    // Get the puzzle for this puzzle id.
    let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&request.id).await?;
    let puzzle = puzzle.ok_or(ServiceError::from(format!("No such puzzle {}", request.id)))?;
    let card = card.unwrap_or(Card::new(&request.id, Local::now().fixed_offset(), state.app_config.srs));

    // Update the user's rating.
    let new_rating = state.user_service.update_rating(user_id, difficulty, GameResult {
        rating: puzzle.rating,
        deviation: puzzle.rating_deviation,
        score: difficulty.score(),
    }).await?;

    // Review the card.
    state.tactics_service.apply_review(user_id, new_rating, card, difficulty).await?;

    Ok(())
}

/// GET /api/tactics/review.
pub async fn next_review(
    State(state): State<AppState>
) -> Result<Json<Option<CardResponse>>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let _user_id = UserService::local_user_id();

    let response = state.tactics_service
        .get_next_review()
        .await?
        .map(|(puzzle, card)| {
            CardResponse { puzzle, card, due_today: true }
        });

    Ok(Json::from(response))
}

/// GET /api/tactics/by_id/:puzzle_id.
pub async fn puzzle_by_id(
    State(state): State<AppState>,
    Path(puzzle_id): Path<String>,
) -> Result<Json<Option<CardResponse>>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let _user_id = UserService::local_user_id();

    let (puzzle, card) = state.tactics_service
        .get_puzzle_by_id(puzzle_id.as_str())
        .await?;

    let response = match puzzle {
        Some(puzzle) => {
            let now = Local::now().fixed_offset();
            let card = card.unwrap_or(Card::new(&puzzle_id, now, state.app_config.srs));
            let due_today = card.is_due::<LocalTimeProvider>();
            Some(CardResponse { puzzle, card, due_today })
        },
        _ => None,
    };

    Ok(Json::from(response))
}

/// GET /api/tactics/random/:min_rating/:max_rating.
pub async fn random_puzzle(
    State(state): State<AppState>,
    Path((min_rating, max_rating)): Path<(i64, i64)>,
) -> Result<Json<Option<CardResponse>>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let _user_id = UserService::local_user_id();

    let (puzzle, card) = state.tactics_service
        .get_random_puzzle(min_rating, max_rating)
        .await?;

    let response = match puzzle {
        Some(puzzle) => {
            let now = Local::now().fixed_offset();
            let card = card.unwrap_or(Card::new(&puzzle.puzzle_id, now, state.app_config.srs));
            let due_today = card.is_due::<LocalTimeProvider>();
            Some(CardResponse { card, puzzle, due_today })
        },
        _ => None,
    };

    Ok(Json::from(response))
}
