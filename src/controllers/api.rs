use serde_json::Value;

use crate::services::ServiceError;
use crate::services::user_service::UserService;
use crate::util;

/// A debug endpoint that resets the user's rating to a specified value.
/// TODO: add this into the settings page, or something like that.
pub async fn reset_user_rating(user_service: UserService, new_rating: i64)
    -> Result<impl warp::Reply, warp::Rejection>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    log::info!("Manually resetting user's rating to {new_rating}");
    user_service.reset_user_rating(user_id, new_rating, 250, 0.06)
        .await
        .map(|_| Ok(warp::reply::html(format!("Reset user rating to {new_rating}"))))?
}

/// Get a user's stats.
pub async fn user_stats(user_service: UserService)
    -> Result<warp::reply::Json, warp::Rejection>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let user_rating = user_service.get_user_rating(user_id).await?;
    let stats = user_service.get_user_stats(user_id).await?;

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

    Ok(warp::reply::json(&json_stats))
}

/// Get a user's review forecast.
pub async fn review_forecast(user_service: UserService, length_days: i64)
    -> Result<warp::reply::Json, warp::Rejection>
{
    const MIN_LENGTH_DAYS: i64 = 0;
    const MAX_LENGTH_DAYS: i64 = 30;

    if length_days < MIN_LENGTH_DAYS || length_days > MAX_LENGTH_DAYS {
        Err(ServiceError::InvalidParameter("length_days".to_string()))?;
    }

    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let review_forecast = user_service.get_review_forecast(user_id, length_days).await?;

    Ok(warp::reply::json(&review_forecast))
}

/// Get a user's rating history.
pub async fn rating_history(user_service: UserService)
    -> Result<warp::reply::Json, warp::Rejection>
{
    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let rating_history = user_service.get_rating_history(user_id).await?
        .into_iter()
        .map(|(date, rating)| (date.to_rfc3339(), rating))
        .collect::<Vec<_>>();

    Ok(warp::reply::json(&rating_history))
}

/// Get a user's review histogram with the specified bucket size.
pub async fn review_score_histogram(user_service: UserService, bucket_size: i64)
    -> Result<warp::reply::Json, warp::Rejection>
{
    const MIN_BUCKET_SIZE: i64 = 50;

    if bucket_size < MIN_BUCKET_SIZE {
        Err(ServiceError::InvalidParameter("bucket_size".to_string()))?;
    }

    // TODO: use a JWT to get the user_id.
    let user_id = UserService::local_user_id();

    let json_data = Value::Array(user_service
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

    Ok(warp::reply::json(&json_data))
}
