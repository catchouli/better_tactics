use askama::Template;

use crate::route::BaseTemplateData;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplateData,
}

pub async fn index_page()
    -> Result<IndexTemplate, warp::Rejection>
{
    Ok(IndexTemplate {
        base: Default::default(),
    })
}
