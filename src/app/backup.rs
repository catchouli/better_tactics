use std::path::Path;
use std::error::Error;

use crate::app::AppConfig;
use crate::db::PuzzleDatabase;
use crate::time::{LocalTimeProvider, TimeProvider};

/// Run the backup if a daily backup hasn't been created yet today.
pub async fn run_backup(app_config: AppConfig, db: PuzzleDatabase)
    -> Result<(), Box<dyn Error>>
{
    type TP = LocalTimeProvider;

    if !app_config.backup.enabled {
        log::warn!("run_backup called but backups are not enabled");
        return Ok(());
    }

    let mut app_data = db.get_app_data("").await?
        .ok_or_else(|| "Failed to get app_data when trying to run backup")?;

    // Check if the last 
    let cur_time = TP::now();
    if let Some(last_backup_date) = app_data.last_backup_date {
        let scheduled = app_config.last_backup_datetime();
        if last_backup_date >= scheduled {
            return Ok(());
        }

        log::info!("Backing up database (last_backup_date: {last_backup_date})");
    }
    else {
        log::info!("Backing up database (no last_backup_date)");
    }

    // Run backup.
    let cur_time_str = cur_time.format("%Y-%m-%d_%H-%M-%S").to_string();
    let backup_db_path = Path::new(&app_config.backup.path)
        .join(format!("backup_{cur_time_str}.sqlite"));
    db.backup_database(&backup_db_path.to_string_lossy()).await?;

    // Store last backup time.
    log::info!("Storing last backup time");
    app_data.last_backup_date = Some(cur_time.fixed_offset());
    db.set_app_data(&app_data).await?;

    Ok(())
}
