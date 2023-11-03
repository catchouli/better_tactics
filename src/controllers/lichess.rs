use askama::Template;

use super::BaseTemplateData;

/// The template for displaying the lichess page.
#[derive(Template)]
#[template(path = "retry-lichess-puzzles.html")]
pub struct LichessTemplate {
    base: BaseTemplateData,
}

pub async fn lichess_page() -> LichessTemplate
{
    LichessTemplate {
        base: Default::default(),
    }
}
