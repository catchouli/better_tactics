use std::collections::HashMap;
use std::fs::File;
use std::io::{Write, SeekFrom, Seek};
use csv::StringRecord;
use futures::StreamExt;

use crate::db::{PuzzleDatabase, Puzzle, Opening, Theme, AddPuzzleTheme};

/// Initialise the puzzle db if necessary.
pub async fn init_db(db: PuzzleDatabase) -> Result<(), String> {
    let app_data = db.get_app_data("").await
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
        let puzzle_count = db.get_puzzle_count().await
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
    const BYTES_PER_PROGRESS_REPORT: usize = 25 * BYTES_PER_MEGABYTE;

    // If the puzzle db is just already in the current working directory, just use that.
    if let Ok(file) = File::open(LICHESS_DB_NAME) {
        return Ok(file);
    }

    // Otherwise, create a temporary file and download it.
    let mut file = tempfile::tempfile()
        .map_err(|e| format!("Failed to create temp file: {e}"))?;

    log::info!("Downloading {LICHESS_DB_URL}");

    let client = reqwest::Client::builder()
        .user_agent(crate::app::APP_USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to create reqwest client: {e}"))?;

    let response = client.get(LICHESS_DB_URL).send().await
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
async fn import_lichess_database(mut db: PuzzleDatabase, lichess_db_raw: File)
    -> Result<(), String>
{
    /// The total number of puzzles in the database. It's a shame to have to have this hardcoded
    /// here, but there's no easy way to tell as we're reading it since we're streaming it from a
    /// file, and this should work in the majority of cases. If it ever changes significantly, we
    /// can just update it.
    const TOTAL_PUZZLES: usize = 3_500_000;

    const PUZZLES_PER_IMPORT_BATCH: usize = 10000;
    const PUZZLES_PER_PROGRESS_UPDATE: usize = 100000;

    // We expect 10 rows per puzzle entry.
    const EXPECTED_ROWS: usize = 10;

    log::info!("Importing lichess puzzle database in background...");

    // Get the source identifier for lichess puzzles.
    let lichess_puzzle_source = db.get_puzzle_source("lichess").await
        .map_err(|e| format!("Failed to get lichess puzzle source: {e}"))?
        .ok_or_else(|| format!("No lichess puzzle source"))?;

    if let Ok(decoder) = zstd::stream::Decoder::new(lichess_db_raw) {
        let mut csv_reader = csv::Reader::from_reader(decoder);

        let mut puzzles_imported = 0;
        let mut last_report = 0;

        // Hashmaps for storing theme and opening ids.
        let mut themes: HashMap<String, Theme> = HashMap::new();
        let mut openings: HashMap<String, Opening> = HashMap::new();

        // Temporary storage for puzzles we have in memory ready to import.
        let mut puzzles = Vec::new();
        let mut puzzle_themes: Vec<AddPuzzleTheme> = Vec::new();
        let mut puzzle_openings: Vec<AddPuzzleTheme> = Vec::new();

        let mut record: StringRecord = StringRecord::new();

        loop {
            let read_record = csv_reader.read_record(&mut record).map_err(|e| {
                format!("CSV parse error when importing lichess puzzles database: {e}")
            })?;

            if !read_record {
                break;
            }

            if record.len() != EXPECTED_ROWS {
                log::warn!("Skipping record with {} entries, expected at least {}",
                    record.len(), EXPECTED_ROWS);
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
            let game_url = record[8].to_string();

            puzzles.push(Puzzle {
                id: None,
                source: lichess_puzzle_source.id,
                source_id: puzzle_id.clone(),
                fen,
                moves,
                rating,
                rating_deviation,
                popularity,
                number_of_plays,
                game_url,
            });
            puzzles_imported += 1;

            // Parse puzzle themes.
            for theme_name in record[7].to_string().split_whitespace() {
                let theme_id = if let Some(theme) = themes.get(theme_name) {
                    theme.id
                }
                else if let Some(theme) = db.get_theme(theme_name).await.map_err(|e| e.to_string())? {
                    let theme_id = theme.id;
                    themes.insert(theme_name.into(), theme);
                    theme_id
                }
                else {
                    db.add_theme(theme_name).await.map_err(|e| e.to_string())?;
                    let theme = db.get_theme(theme_name).await.map_err(|e| e.to_string())?.unwrap();
                    let theme_id = theme.id;
                    themes.insert(theme_name.into(), theme);
                    theme_id
                };

                puzzle_themes.push(AddPuzzleTheme {
                    source: lichess_puzzle_source.id,
                    source_id: puzzle_id.clone(),
                    theme_id,
                });
            }

            // Parse puzzle openings.
            for opening_name in record[7].to_string().split_whitespace() {
                let opening_id = if let Some(opening) = openings.get(opening_name) {
                    opening.id
                }
                else if let Some(opening) = db.get_opening(opening_name).await.map_err(|e| e.to_string())? {
                    let opening_id = opening.id;
                    openings.insert(opening_name.into(), opening);
                    opening_id
                }
                else {
                    db.add_opening(opening_name).await.map_err(|e| e.to_string())?;
                    let opening = db.get_opening(opening_name).await.map_err(|e| e.to_string())?.unwrap();
                    let opening_id = opening.id;
                    openings.insert(opening_name.into(), opening);
                    opening_id
                };

                puzzle_openings.push(AddPuzzleTheme {
                    source: lichess_puzzle_source.id,
                    source_id: puzzle_id.clone(),
                    theme_id: opening_id,
                });
            }

            // Bulk insert if we have enough.
            if puzzles.len() >= PUZZLES_PER_IMPORT_BATCH {
                if puzzles_imported == PUZZLES_PER_IMPORT_BATCH {
                    log::info!("Importing first puzzle batch...");
                }

                db.add_puzzles(&puzzles).await
                    .map_err(|e| format!("Failed to add puzzles to db: {e}"))?;
                db.add_puzzle_themes(&puzzle_themes).await
                    .map_err(|e| format!("Failed to add puzzle themes to db: {e}"))?;
                db.add_puzzle_openings(&puzzle_openings).await
                    .map_err(|e| format!("Failed to add puzzle openings to db: {e}"))?;

                puzzles.clear();
                puzzle_openings.clear();
                puzzle_themes.clear();
            }

            if puzzles_imported - last_report >= PUZZLES_PER_PROGRESS_UPDATE {
                last_report = puzzles_imported;

                // Calculate imported percent.
                let percent = 100.0 * puzzles_imported as f64 / TOTAL_PUZZLES as f64;
                // Round it to the nearest .5.
                let percent = f64::floor(2.0 * percent) / 2.0;

                log::info!("Puzzle database: {puzzles_imported} puzzles ({percent:.1}%) imported...");
            }
        }

        // Add last batch (should be less than the batch size or it'll be empty).
        if !puzzles.is_empty() {
            db.add_puzzles(&puzzles).await
                .map_err(|e| format!("Failed to add puzzles to db: {e}"))?;
            puzzles.clear();
        }

        // Update flag in db to say the puzzles table is initialised.
        let mut app_data = db.get_app_data("").await
            .map_err(|e| format!("Failed to get app data: {e}"))?
            .ok_or_else(|| format!("No app_data row in database"))?;
        app_data.lichess_db_imported = true;
        db.set_app_data(&app_data).await
            .map_err(|e| format!("Failed to update app data: {e}"))?;

        log::info!("Finished importing {puzzles_imported} puzzles");
    }

    Ok(())
}

