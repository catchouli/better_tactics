#![feature(associated_type_bounds)]
mod db;
mod route;
mod controllers;
mod util;
mod srs;
mod rating;
mod config;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Write, SeekFrom, Seek};
use std::sync::Arc;
use futures::StreamExt;
use tokio::sync::Mutex;

use crate::db::PuzzleDatabase;
use crate::config::AppConfig;

/// Download the lichess puzzles db as a temporary file.
async fn download_puzzle_db() -> Result<File, Box<dyn Error>> {
    const LICHESS_DB_URL: &'static str = "https://database.lichess.org/lichess_db_puzzle.csv.zst";

    let mut file = tempfile::tempfile()?;

    log::info!("Downloading {LICHESS_DB_URL}");
    let response = reqwest::get(LICHESS_DB_URL).await?;
    let mut response_stream = response.bytes_stream();

    let mut counter = 0;
    while let Some(v) = response_stream.next().await {
        let bytes = v?;
        counter += bytes.len();
        file.write_all(&bytes)?;
    }

    log::info!("Downloaded {counter} bytes");

    Ok(file)
}

/// Open the puzzle database and initialize it if needed.
async fn init_db(config: &AppConfig) -> Result<PuzzleDatabase, Box<dyn Error>> {
    // Open puzzle database.
    let mut puzzle_db = PuzzleDatabase::open(&config.db_name)?;

    // If no puzzles are loaded into the db yet, initialise it from a copy of the lichess database.
    let puzzle_count = puzzle_db.get_puzzle_count()?;
    if puzzle_count == 0 {
        log::info!("Puzzle database empty, initialising from lichess puzzles database");

        // Download the lichess database.
        let mut lichess_db = download_puzzle_db().await?;
        lichess_db.seek(SeekFrom::Start(0))?;

        // Initialise our database with it.
        puzzle_db.import_lichess_database(lichess_db)?;
        log::info!("Done initialising puzzle database");
    }
    else {
        log::info!("Loaded puzzle database with {puzzle_count} puzzles");
    }

    Ok(puzzle_db)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set RUST_LOG to info by default for other peoples' convenience.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::builder().init();
    log::info!("Better tactics starting!");

    // Load app config.
    let app_config = AppConfig::from_env()?;
    log::info!("{app_config:?}");

    // Initialise puzzle database.
    let puzzle_db = Arc::new(Mutex::new(init_db(&app_config).await?));

    // Create routes and serve service
    let routes = route::routes(puzzle_db);
    warp::serve(routes).run((app_config.bind_interface, app_config.bind_port)).await;

    Ok(())
}
