use askama::Template;

use crate::route::BaseTemplateData;

/// The about page.
#[derive(Template, Default)]
#[template(path = "about.html")]
pub struct AboutTemplate {
    base: BaseTemplateData,
}

pub fn about_page() -> AboutTemplate {
    AboutTemplate {
        ..Default::default()
    }
}
