use askama::Template;
use chrono::NaiveDate;

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
    rating_history: Vec<(NaiveDate, i64)>,
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

    // Get the rating history as json.
    // TODO: use an api endpoint to provide this instead of baking it into the template.
    fn rating_history_json(&self) -> String {
        use serde_json::Value;
        Value::Array(self.rating_history
            .iter()
            .map(|(date, rating)| {
                let mut map = serde_json::Map::new();
                map.insert("date".to_string(), Value::String(format!("{date}")));
                map.insert("rating".to_string(), Value::Number((*rating).into()));
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

    Ok(IndexTemplate {
        base: Default::default(),
        user_rating: user_service.get_user_rating(user_id).await?,
        stats: user_service.get_user_stats(user_id).await?,
        review_forecast: user_service.get_review_forecast(user_id).await?,
        rating_history: user_service.get_rating_history(user_id).await?,
    })
}
