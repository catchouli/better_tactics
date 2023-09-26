use askama::Template;

/// The template for displaying the index page.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;
