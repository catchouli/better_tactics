use askama::Template;

use super::BaseTemplateData;

/// The about page.
#[derive(Template, Default)]
#[template(path = "about.html")]
pub struct AboutTemplate {
    base: BaseTemplateData,
}

pub async fn about_page() -> AboutTemplate {
    AboutTemplate {
        ..Default::default()
    }
}
