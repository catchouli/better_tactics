use askama::Template;
use chrono::{DateTime, FixedOffset, Local, Duration};

use crate::db::ReviewScoreBucket;
use crate::rating::Rating;
use crate::route::BaseTemplateData;
use crate::services::user_service::{UserService, Stats};
use crate::util;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplateData,
    stats: Stats,
    user_rating: Rating,
    review_forecast: Vec<i64>,
    rating_history: Vec<(DateTime<FixedOffset>, i64)>,
    review_score_histogram: Vec<ReviewScoreBucket>,
}

/// The bucket size for the review score histogram.
const REVIEW_SCORE_HISTOGRAM_BUCKET_SIZE: i64 = 50;

/// The maximum number of days to show in the review forecast.
const REVIEW_FORECAST_LENGTH: i64 = 8;

impl IndexTemplate {
    // Format the review forecast as a javascript array.
    fn review_forecast(&self) -> String {
        let values = self.review_forecast.iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(",");

        format!("[{values}]")
    }

    // Get the rating history as json.
    // TODO: use an api endpoint to provide this instead of baking it into the template.
    fn rating_history_json(&self) -> String {
        use serde_json::Value;
        Value::Array(self.rating_history
            .iter()
            .map(|&(date, rating)| {
                let mut map = serde_json::Map::new();
                map.insert("date".into(), date.to_rfc3339().into());
                map.insert("rating".into(), rating.into());
                Value::Object(map)
            })
            .collect()
        ).to_string()
    }

    // Get the review histogram as json.
    // TODO: use an api endpoint to provide this instead of baking it into the template.
    fn review_histogram_json(&self) -> String {
        use serde_json::Value;
        Value::Array(self.review_score_histogram
            .iter()
            .map(|bucket| {
                let mut map = serde_json::Map::new();
                map.insert("puzzle_rating_min".into(), bucket.puzzle_rating_min.into());
                map.insert("puzzle_rating_max".into(), bucket.puzzle_rating_max.into());
                map.insert("difficulty".into(), bucket.difficulty.to_i64().into());
                map.insert("review_count".into(), bucket.review_count.into());
                Value::Object(map)
            })
            .collect()
        ).to_string()
    }
}

pub async fn index_page(user_service: UserService)
    -> Result<IndexTemplate, warp::Rejection>
{
    let user_id = UserService::local_user_id();

    let user_rating = user_service.get_user_rating(user_id).await?;
    let mut rating_history = user_service.get_rating_history(user_id).await?;

    // Push the current rating to the end so there's some data to show even if the user doesn't
    // have any historical rating values.
    rating_history.push((Local::now().fixed_offset(), user_rating.rating));
    rating_history.push((Local::now().fixed_offset() + Duration::seconds(1), user_rating.rating));

    let review_score_histogram = user_service.get_review_score_histogram(user_id,
        REVIEW_SCORE_HISTOGRAM_BUCKET_SIZE).await?;

    Ok(IndexTemplate {
        base: Default::default(),
        user_rating,
        stats: user_service.get_user_stats(user_id).await?,
        review_forecast: user_service.get_review_forecast(user_id, REVIEW_FORECAST_LENGTH).await?,
        rating_history,
        review_score_histogram,
    })
}
