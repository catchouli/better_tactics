use axum::Router;
use axum::http::{Uri, header, StatusCode};
use axum::response::{Response, IntoResponse};
use rust_embed::RustEmbed;

/// The static assets path.
pub static STATIC_ASSETS_PATH: &str = concat!("/assets_", env!("CARGO_PKG_VERSION"));

/// The embedded assets.
#[derive(RustEmbed)]
#[folder = "assets"]
struct Asset;

pub fn routes() -> Router {
    Router::new().fallback(handler)
}

/// The assets handler.
async fn handler(uri: Uri) -> Response {
    let path = uri.path()
        .trim_start_matches(STATIC_ASSETS_PATH)
        .trim_start_matches('/');

    if let Some(content) = Asset::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
    }
    else {
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    }
}
