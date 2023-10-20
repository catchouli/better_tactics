use chrono::Duration;
use tokio::sync::Mutex;
use std::path::Path;
use std::sync::Arc;
use std::error::Error;

use crate::app::AppConfig;
use crate::db::PuzzleDatabase;
use crate::srs;
use crate::time::{LocalTimeProvider, TimeProvider};

/// Run the backup if a daily backup hasn't been created yet today.
pub async fn run_backup(app_config: AppConfig, db: Arc<Mutex<PuzzleDatabase>>)
    -> Result<(), Box<dyn Error>>
{
    type TP = LocalTimeProvider;

    if !app_config.backup.enabled {
        log::warn!("run_backup called but backups are not enabled");
        return Ok(());
    }

    let db = db.lock().await;
    let mut app_data = db.get_app_data("").await?
        .ok_or_else(|| "Failed to get app_data when trying to run backup")?;

    // Check if the last 
    let cur_time = TP::now();
    if let Some(last_backup_date) = app_data.last_backup_date {
        let previous_day_end = srs::day_end_datetime::<TP>() - Duration::days(1);

        if last_backup_date > previous_day_end {
            return Ok(());
        }

        log::info!("last_backup_date ({}) is before the start of the day ({}), backing up database...",
                   last_backup_date, previous_day_end);
    }
    else {
        log::info!("last_backup_date is not set, backing up database...");
    }

    // Run backup.
    let cur_time_str = cur_time.format("%Y-%m-%d_%H-%M-%S").to_string();
    let backup_db_path = Path::new(&app_config.backup.path).join(&cur_time_str);
    db.backup_database(&backup_db_path.to_string_lossy()).await?;

    // Store last backup time.
    log::info!("Storing last backup time");
    app_data.last_backup_date = Some(cur_time);
    db.set_app_data(&app_data).await?;

    Ok(())
}
