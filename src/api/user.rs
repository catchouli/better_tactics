use axum::extract::{State, Json, Path};
use serde_json::Value;

use crate::api::{ApiError, ApiResponse};
use crate::app::AppState;
use crate::services::user_service::UserService;
use crate::util;

/// Reset the user's rating to the specified value.
/// TODO: add this into the settings page.
pub async fn reset_rating(
    State(state): State<AppState>,
    Path(new_rating): Path<i64>,
) -> Result<ApiResponse, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    log::info!("Manually resetting user's rating to {new_rating}");
    state.user_service.reset_user_rating(user_id, new_rating, 250, 0.06)
        .await?;

    Ok(ApiResponse {
        response: format!("Reset user rating to {new_rating}"),
    })
}

/// Get a user's stats.
pub async fn stats(State(state): State<AppState>)
    -> Result<Json<serde_json::Value>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let user_rating = state.user_service.get_user_rating(user_id).await?;
    let stats = state.user_service.get_user_stats(user_id).await?;

    let json_stats = Value::Object({
        let mut map = serde_json::Map::new();
        map.insert("user_rating".into(), Value::Object({
            let mut map = serde_json::Map::new();
            map.insert("rating".into(), user_rating.rating.into());
            map.insert("deviation".into(), user_rating.deviation.into());
            map.insert("volatility".into(), user_rating.volatility.into());
            map
        }));
        map.insert("card_count".into(), stats.card_count.into());
        map.insert("review_count".into(), stats.review_count.into());
        map.insert("reviews_due_now".into(), stats.reviews_due_now.into());
        map.insert("reviews_due_today".into(), stats.reviews_due_today.into());
        let next_review_due = util::maybe_review_timestamp_to_human(&stats.next_review_due);
        map.insert("next_review_due".into(), next_review_due.into());
        map
    });

    Ok(json_stats.into())
}

/// Get a user's review forecast.
pub async fn review_forecast(
    State(state): State<AppState>,
    Path(length_days): Path<i64>,
) -> Result<Json<Vec<(i64, i64)>>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let review_forecast = state.user_service.get_review_forecast(user_id, length_days).await?;

    Ok(review_forecast.into())
}

/// Get a user's rating history.
pub async fn rating_history(State(state): State<AppState>)
    -> Result<Json<Vec<(String, i64)>>, ApiError>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let rating_history = state.user_service.get_rating_history(user_id).await?
        .into_iter()
        .map(|(date, rating)| (date.to_rfc3339(), rating))
        .collect::<Vec<_>>();

    Ok(rating_history.into())
}

/// Get a user's review histogram with the specified bucket size.
pub async fn review_score_histogram(
    State(state): State<AppState>,
    Path(bucket_size): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError>
{
    const MIN_BUCKET_SIZE: i64 = 50;

    if bucket_size < MIN_BUCKET_SIZE {
        Err(ApiError::InvalidParameter(format!("bucket_size must be greater than {}",
                                               MIN_BUCKET_SIZE)))?;
    }

    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let json_data = Value::Array(state.user_service
        .get_review_score_histogram(user_id, bucket_size)
        .await?
        .into_iter()
        .map(|bucket| {
            let mut map = serde_json::Map::new();
            map.insert("puzzle_rating_min".into(), bucket.puzzle_rating_min.into());
            map.insert("puzzle_rating_max".into(), bucket.puzzle_rating_max.into());
            map.insert("difficulty".into(), bucket.difficulty.to_i64().into());
            map.insert("review_count".into(), bucket.review_count.into());
            Value::Object(map)
        })
        .collect()
    );

    Ok(json_data.into())
}
