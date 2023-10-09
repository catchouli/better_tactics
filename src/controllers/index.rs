use std::sync::Arc;

use askama::Template;
use chrono::Duration;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Stats, User};
use crate::route::{InternalError, BaseTemplateData};
use crate::srs;
use crate::time::LocalTimeProvider;
use crate::util;

/// The maximum number of days to show in the review forecast.
const REVIEW_FORECAST_LENGTH: usize = 8;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplateData,
    user: User,
    stats: Stats,
    review_forecast: Vec<i64>,
}

impl IndexTemplate {
    // Format the review forecast as a javascript array.
    fn review_forecast(&self) -> String {
        let values = self.review_forecast.iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(",");

        format!("[{values}]")
    }
}

pub async fn index_page(puzzle_db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<IndexTemplate, warp::Rejection>
{
    let user_id = PuzzleDatabase::local_user_id();
    let puzzle_db = puzzle_db.lock().await;

    // Get user details and stats.
    let user = puzzle_db.get_user_by_id(user_id).await
        .map_err(InternalError::from)?
        .ok_or(InternalError::new(format!("No local user record in database")))?;

    let review_cutoff = srs::day_end_datetime::<LocalTimeProvider>();
    let stats = puzzle_db.get_user_stats(user_id, review_cutoff.clone())
        .await.map_err(InternalError::from)?;

    // Get the reviews due per day for the review forecast.
    let review_forecast = {
        let mut review_forecast = Vec::new();
        review_forecast.push(stats.reviews_due_today);

        let mut start = review_cutoff;
        for _ in 0..REVIEW_FORECAST_LENGTH {
            let end = start + Duration::days(1);
            let reviews_due = puzzle_db.reviews_due_between(start, end)
                .await.map_err(InternalError::from)?;
            review_forecast.push(reviews_due);
            start = end;
        }

        review_forecast
    };

    Ok(IndexTemplate {
        base: Default::default(),
        user,
        stats,
        review_forecast,
    })
}
