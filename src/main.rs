mod db;
mod route;
mod controllers;
mod util;
mod srs;
mod rating;
mod config;
mod time;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Write, SeekFrom, Seek};
use std::sync::Arc;
use futures::StreamExt;
use tokio::sync::Mutex;

use crate::db::{PuzzleDatabase, Puzzle};
use crate::config::AppConfig;

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

    // Open puzzle database.
    let puzzle_db = Arc::new(Mutex::new(PuzzleDatabase::open(&app_config.db_name).await?));

    // Initialise puzzle database.
    tokio::spawn({
        let puzzle_db = puzzle_db.clone();
        async move {
            if let Err(e) = init_db(puzzle_db.clone()).await {
                log::error!("{e}");
            }
        }
    });

    // Create routes and serve service.
    let routes = route::routes(puzzle_db);
    let server_task = warp::serve(routes).run((app_config.bind_interface, app_config.bind_port));

    tokio::join!(server_task);

    Ok(())
}

/// Open the puzzle database and initialize it if needed.
async fn init_db(db: Arc<Mutex<PuzzleDatabase>>) -> Result<(), String> {
    let app_data = db.lock().await.get_app_data("").await
        .map_err(|e| format!("Failed to get app data: {e}"))?
        .ok_or_else(|| format!("Internal error: no app_data row in database"))?;

    // If the puzzle db hasn't been fully initialised yet, download it.
    if !app_data.lichess_db_imported {
        log::info!(
            "Puzzle database not fully initialised, initialising from lichess puzzles database in background");

        // Download the lichess database.
        let mut lichess_db = download_puzzle_db().await
            .map_err(|e| format!("Failed to download lichess puzzle database: {e}"))?;
        lichess_db.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek lichess db file: {e}"))?;

        // Initialise our database with it.
        import_lichess_database(db, lichess_db).await
            .map_err(|e| format!("Failed to import lichess puzzle db: {e}"))?;
    }
    else {
        let puzzle_count = db.lock().await.get_puzzle_count().await
            .map_err(|e| format!("Failed to get puzzle count: {e}"))?;
        log::info!("Loaded puzzle database with {puzzle_count} puzzles");
    }

    Ok(())
}

/// Download the lichess puzzles db as a temporary file.
async fn download_puzzle_db() -> Result<File, String> {
    const LICHESS_DB_NAME: &'static str = "lichess_db_puzzle.csv.zst";
    const LICHESS_DB_URL: &'static str = "https://database.lichess.org/lichess_db_puzzle.csv.zst";

    const BYTES_PER_MEGABYTE: usize = 1024 * 1024;
    const BYTES_PER_PROGRESS_REPORT: usize = 10 * BYTES_PER_MEGABYTE;

    // If the puzzle db is just already in the current working directory, just use that.
    if let Ok(file) = File::open(LICHESS_DB_NAME) {
        return Ok(file);
    }

    // Otherwise, create a temporary file and download it.
    let mut file = tempfile::tempfile()
        .map_err(|e| format!("Failed to create temp file: {e}"))?;

    log::info!("Downloading {LICHESS_DB_URL}");
    let response = reqwest::get(LICHESS_DB_URL).await
        .map_err(|e| format!("Failed to request lichess puzzle db: {e}"))?;

    let total_length_mb = response.content_length()
        .map(|bytes| (bytes as usize / BYTES_PER_MEGABYTE).to_string())
        .unwrap_or("?".to_string());

    let mut response_stream = response.bytes_stream();

    let mut counter = 0;
    let mut bytes_since_reported = 0;
    while let Some(v) = response_stream.next().await {
        let bytes = v
            .map_err(|e| format!("Failed to read byte stream: {e}"))?;
        counter += bytes.len();
        bytes_since_reported += bytes.len();

        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write bytes to temp file: {e}"))?;

        while bytes_since_reported > BYTES_PER_PROGRESS_REPORT {
            log::info!("Lichess puzzle database: {}/{}MB downloaded",
                       counter / BYTES_PER_MEGABYTE,
                       total_length_mb);
            bytes_since_reported -= BYTES_PER_PROGRESS_REPORT;
        }
    }

    log::info!("Downloaded {counter} bytes");

    Ok(file)
}

/// Import lichess database from file.
async fn import_lichess_database(db: Arc<Mutex<PuzzleDatabase>>, lichess_db_raw: File)
    -> Result<(), String>
{
    const MAX_PUZZLES_TO_IMPORT: usize = 10_000_000;
    const PUZZLES_PER_PROGRESS_UPDATE: usize = 2500;

    log::info!("Importing lichess puzzle database");

    if let Ok(decoder) = zstd::stream::Decoder::new(lichess_db_raw) {
        let mut csv_reader = csv::Reader::from_reader(decoder);
        let mut puzzles_imported = 0;

        let mut puzzles = Vec::new();

        for result in csv_reader.records().take(MAX_PUZZLES_TO_IMPORT) {
            const EXPECTED_ROWS: usize = 10;

            // Unwrap record.
            let record = result.map_err(|e|
                format!("CSV parse error when importing lichess puzzles database: {e}")
            )?;

            if record.len() != EXPECTED_ROWS {
                log::warn!("Skipping record with {} entries, expected at least {}", record.len(), EXPECTED_ROWS);
                continue;
            }

            // Import puzzle.
            let puzzle_id = record[0].to_string();
            let fen = record[1].to_string();
            let moves = record[2].to_string();
            let rating = record[3].parse()
                .map_err(|e| format!("Failed to parse rating field {e}"))?;
            let rating_deviation = record[4].parse()
                .map_err(|e| format!("Failed to parse rating_deviation field {e}"))?;
            let popularity = record[5].parse()
                .map_err(|e| format!("Failed to parse popularity field {e}"))?;
            let number_of_plays = record[6].parse()
                .map_err(|e| format!("Failed to parse number_of_plays field {e}"))?;
            let themes = record[7].to_string().split_whitespace().map(ToString::to_string).collect();
            let game_url = record[8].to_string();
            let opening_tags = record[9].to_string().split_whitespace().map(ToString::to_string).collect();

            puzzles.push(Puzzle {
                puzzle_id,
                fen,
                moves,
                rating,
                rating_deviation,
                popularity,
                number_of_plays,
                themes,
                game_url,
                opening_tags,
            });
            puzzles_imported += 1;

            // Bulk insert if we have enough.
            if puzzles.len() >= PUZZLES_PER_PROGRESS_UPDATE {
                db.lock().await.add_puzzles(&puzzles).await
                    .map_err(|e| format!("Failed to add puzzles to db: {e}"))?;
                puzzles.clear();
                log::info!("Progress: {puzzles_imported} puzzles imported...");
            }
        }

        // Add last batch (should be less than the batch size or it'll be empty).
        if !puzzles.is_empty() {
            db.lock().await.add_puzzles(&puzzles).await
                .map_err(|e| format!("Failed to add puzzles to db: {e}"))?;
            puzzles.clear();
        }

        // Update flag in db to say the puzzles table is initialised.
        let mut app_data = db.lock().await.get_app_data("").await
            .map_err(|e| format!("Failed to get app data: {e}"))?
            .ok_or_else(|| format!("No app_data row in database"))?;
        app_data.lichess_db_imported = true;
        db.lock().await.set_app_data(&app_data).await
            .map_err(|e| format!("Failed to update app data: {e}"))?;

        log::info!("Finished importing {puzzles_imported} puzzles");
    }

    Ok(())
}

