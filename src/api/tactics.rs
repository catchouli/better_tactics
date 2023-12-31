use axum::extract::{State, Json, Path};
use chrono::Local;
use serde::Deserialize;
use serde::ser::SerializeStruct;

use crate::api::{ApiError, ApiResult};
use crate::db::{Puzzle, PuzzleHistoryEntry};
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
    // The current review count of the card, to prevent a card from being reviewed twice by
    // accident, the client can submit this value to us and we can assume the request has already
    // been fulfilled if it doesn't match.
    pub review_count: i64,
}

/// Request JSON for /api/tactics/random/skip.
#[derive(Debug, Clone, Deserialize)]
pub struct SkipRequest {
    pub id: String,
    pub difficulty: i64,
    pub update_rating: bool,
}

/// Response JSON for endpoints returning a puzzle and a card.
#[derive(Debug, serde::Serialize)]
pub struct CardResponse {
    puzzle: Option<Puzzle>,
    #[serde(serialize_with = "serialize_card")]
    card: Option<Card>,
    due_today: bool,
}

/// Response JSON for puzzle history.
#[derive(Debug, serde::Serialize)]
pub struct PuzzleHistoryResponse {
    current_page: u64,
    num_pages: u64,
    puzzles: Vec<PuzzleHistoryEntry>,
}

fn serialize_card<S: serde::Serializer>(card: &Option<Card>, serializer: S) -> Result<S::Ok, S::Error> {
    if let Some(card) = card {
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
    else {
        serializer.serialize_none()
    }
}

/// POST /api/tactics/review.
pub async fn review(
    State(mut state): State<AppState>,
    Json(request): Json<ReviewRequest>,
) -> ApiResult<()>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let difficulty = Difficulty::from_i64(request.difficulty)
        .map_err(|_| ApiError::InvalidParameter("difficulty".into()))?;

    // If this is for the user's saved next puzzle, clear it.
    if let Some(next_saved_puzzle) = state.user_service.get_user_next_puzzle(user_id).await? {
        if next_saved_puzzle == request.id {
            state.user_service.set_user_next_puzzle(user_id, None).await?;
        }
    }

    // Get the puzzle for this puzzle id.
    let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&request.id).await?;
    let puzzle = puzzle.ok_or(ServiceError::from(format!("No such puzzle {}", request.id)))?;
    let card = card.unwrap_or(Card::new(&request.id, Local::now().fixed_offset(), state.app_config.srs));

    if card.review_count != request.review_count {
        log::warn!(concat!("Attempted to review card with incorrect review count ({} != {}), it's possible "
            , "this request has accidentally been submitted twice so we're ignoring this attempt"),
            request.review_count, card.review_count);
        return Ok(());
    }

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
) -> ApiResult<Json<CardResponse>>
{
    // TODO: use a JWT to get the user_id.
    let _user_id = UserService::local_user_id();

    let response = state.tactics_service
        .get_next_review()
        .await?
        .map(|(puzzle, card)| {
            CardResponse { puzzle: Some(puzzle), card: Some(card), due_today: true }
        })
        .unwrap_or(CardResponse { puzzle: None, card: None, due_today: false });

    Ok(Json::from(response))
}

/// GET /api/tactics/by_id/:puzzle_id.
pub async fn puzzle_by_id(
    State(state): State<AppState>,
    Path(puzzle_id): Path<String>,
) -> ApiResult<Json<CardResponse>>
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
            CardResponse { puzzle: Some(puzzle), card: Some(card), due_today }
        },
        _ => CardResponse { puzzle: None, card: None, due_today: false },
    };

    Ok(Json::from(response))
}

/// GET /api/tactics/random/:min_rating/:max_rating.
pub async fn random_puzzle(
    State(mut state): State<AppState>,
    Path((min_rating, max_rating)): Path<(i64, i64)>,
) -> ApiResult<Json<CardResponse>>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    // Get the user's stored next puzzle if there is one.
    let saved_next_puzzle = state.user_service.get_user_next_puzzle(user_id).await?;

    if let Some(saved_next_puzzle) = saved_next_puzzle {
        // Check that it's not been done yet, if so we can just return the same one.
        let (puzzle, card) = state.tactics_service.get_puzzle_by_id(&saved_next_puzzle).await?;

        if puzzle.is_some() && card.is_none() {
            return Ok(Json(CardResponse {
                puzzle,
                card: Some(Card::new(&saved_next_puzzle, Local::now().fixed_offset(),
                    state.app_config.srs)),
                due_today: true,
            }));
        }
    }

    // Get the next random puzzle for the user.
    let (puzzle, card) = state.tactics_service
        .get_random_puzzle(min_rating, max_rating)
        .await?;

    let response = match puzzle {
        Some(puzzle) => {
            // Store it so it comes up again next time until it's skipped.
            state.user_service.set_user_next_puzzle(user_id, Some(&puzzle.puzzle_id)).await?;

            let now = Local::now().fixed_offset();
            let card = card.unwrap_or(Card::new(&puzzle.puzzle_id, now, state.app_config.srs));
            let due_today = card.is_due::<LocalTimeProvider>();
            CardResponse { card: Some(card), puzzle: Some(puzzle), due_today }
        },
        _ => CardResponse { card: None, puzzle: None, due_today: false },
    };

    Ok(Json::from(response))
}

pub async fn skip_next(
    State(mut state): State<AppState>,
    Json(request): Json<SkipRequest>,
) -> ApiResult<()>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    log::info!("Update rating: {}", request.update_rating);

    let user_next_puzzle = state.user_service.get_user_next_puzzle(user_id).await?;

    if user_next_puzzle.is_none() {
        log::warn!("Skip requested but user has no next puzzle set");
        return Ok(());
    }
    let user_next_puzzle = user_next_puzzle.unwrap();

    if user_next_puzzle != request.id {
        log::warn!("Skip requested but puzzle wasn't the user's saved next puzzle");
        return Ok(());
    }

    // If the puzzle exists, add the puzzle as skipped, and update the user's rating if
    // requested.
    if let (Some(puzzle), _) = state.tactics_service.get_puzzle_by_id(&user_next_puzzle).await? {
        state.tactics_service.skip_puzzle(user_id, &puzzle).await?;

        if request.update_rating {
            let difficulty = Difficulty::from_i64(request.difficulty)
                .map_err(|_| ApiError::InvalidParameter("request.difficulty".into()))?;

            state.user_service.update_rating(user_id, difficulty, GameResult {
                rating: puzzle.rating,
                deviation: puzzle.rating_deviation,
                score: difficulty.score(),
            }).await?;
        }
    }

    // Clear the value regardless.
    state.user_service.set_user_next_puzzle(user_id, None).await?;

    Ok(())
}

/// GET /api/tactics/history/:page.
pub async fn puzzle_history(
    State(state): State<AppState>,
    Path(page): Path<u64>,
) -> ApiResult<Json<PuzzleHistoryResponse>>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    // The length in puzzles for each page of the puzzle history.
    const PUZZLE_HISTORY_PAGE_LENGTH: u64 = 5;

    // Get history from reviews.
    let count = PUZZLE_HISTORY_PAGE_LENGTH;
    let offset = (page - 1) * count;

    let (puzzles, total_count) = state.tactics_service
        .get_puzzle_history(user_id, offset as i64, count as i64)
        .await?;

    Ok(Json(PuzzleHistoryResponse {
        current_page: page,
        num_pages: total_count as u64 / count + 1,
        puzzles,
    }))
}
