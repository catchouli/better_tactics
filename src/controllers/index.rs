use askama::Template;

use super::BaseTemplateData;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplateData,
}

pub async fn index_page() -> IndexTemplate
{
    IndexTemplate {
        base: Default::default(),
    }
}
