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
use tokio_cron_scheduler::{Job, JobScheduler};

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
    let puzzle_db = PuzzleDatabase::open(&app_config.database_url, app_config.srs).await?;

    // Initialise puzzle database.
    tokio::spawn({
        let puzzle_db = puzzle_db.clone();
        async move {
            if let Err(e) = lichess::init_db(puzzle_db.clone()).await {
                log::error!("{e}");
            }
        }
    });

    // Run backup straight away if necessary.
    if app_config.backup.enabled {
        try_run_backup(app_config.clone(), puzzle_db.clone()).await;
    }

    // Start job scheduler.
    tokio::spawn(start_job_scheduler(app_config.clone(), puzzle_db.clone()));

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

async fn start_job_scheduler(app_config: AppConfig, db: PuzzleDatabase)
    -> Result<(), String>
{
    run_job_scheduler(app_config, db)
        .await
        .map_err(|e| e.to_string())
}

async fn run_job_scheduler(app_config: AppConfig, db: PuzzleDatabase)
    -> Result<(), Box<dyn Error>>
{
    let scheduler = JobScheduler::new().await?;

    if app_config.backup.enabled {
        let backup_job = create_backup_job(app_config, db)?;
        scheduler.add(backup_job).await?;
    }

    Ok(scheduler.start().await?)
}

fn create_backup_job(app_config: AppConfig, db: PuzzleDatabase)
    -> Result<Job, Box<dyn Error>>
{
    // Run backup task at the first second and minute of every hour, the backup already checks if
    // a backup is scheduled so it'll just happen the next time it's needed.
    Ok(Job::new_async("0 0 * * * *", move |_, _| {
        Box::pin(try_run_backup(app_config.clone(), db.clone()))
    })?)
}

async fn try_run_backup(app_config: AppConfig, db: PuzzleDatabase) {
    if let Err(e) = app::backup::run_backup(app_config, db).await {
        log::error!("Error when backing up database: {e}");
    }
}
