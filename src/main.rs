mod api;
mod assets;
mod app;
mod controllers;
mod db;
mod lichess;
mod rating;
mod services;
mod srs;
mod time;
mod util;

use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::PuzzleDatabase;
use crate::app::{AppConfig, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set RUST_LOG to info by default for other peoples' convenience.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::builder().init();
    log::info!("{}", app::APP_USER_AGENT);

    // Load app config.
    let app_config = AppConfig::from_env()?;
    log::info!("{app_config:?}");

    // Open puzzle database.
    let puzzle_db = PuzzleDatabase::open(&app_config.db_name, app_config.srs).await?;
    let puzzle_db = Arc::new(Mutex::new(puzzle_db));

    // Initialise puzzle database.
    tokio::spawn({
        let puzzle_db = puzzle_db.clone();
        async move {
            if let Err(e) = lichess::init_db(puzzle_db.clone()).await {
                log::error!("{e}");
            }
        }
    });

    // Create application routes.
    let app_state = AppState::new(app_config.clone(), puzzle_db);
    let app = controllers::routes(app_state.clone())
        .nest_service("/api", api::routes(app_state))
        .nest_service(assets::STATIC_ASSETS_PATH, assets::routes());

    // Start web server.
    log::info!("The application is now started, access it at {}:{}",
        app_config.bind_interface, app_config.bind_port);

    let socket_addr = SocketAddr::from((app_config.bind_interface, app_config.bind_port));
    axum::Server::bind(&socket_addr).serve(app.into_make_service()).await?;

    Ok(())
}

