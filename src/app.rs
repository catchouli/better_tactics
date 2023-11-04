pub mod backup;

use std::env::{self, VarError};
use std::error::Error;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

use chrono::{NaiveTime, DateTime, Local, Duration};
use url::Url;

use crate::db::PuzzleDatabase;
use crate::services::tactics_service::TacticsService;
use crate::services::user_service::UserService;
use crate::srs::{SrsConfig, ReviewOrder};

/// The application useragent, e.g. "better_tactics/0.0.1".
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// A type for database urls that implements Debug so that we can pretty print the app config
/// without the URLs printing out all their fields.
#[derive(Clone)]
pub struct DatabaseUrl(pub Url);

impl Debug for DatabaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DatabaseUrl(url) = self;
        write!(f, "\"{url}\"")
    }
}

/// The application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_interface: IpAddr,
    pub bind_port: u16,
    pub database_url: DatabaseUrl,
    pub tactics: TacticsConfig,
    pub srs: SrsConfig,
    pub backup: BackupConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone)]
pub struct TacticsConfig {
    pub puzzle_rating_variation_up: f32,
    pub puzzle_rating_variation_down: f32,
}

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub enabled: bool,
    pub path: String,
    pub hour: NaiveTime,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub initial_move_delay: u32,
    pub subsequent_move_delay: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_interface: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            bind_port: 3030,
            database_url: DatabaseUrl(Url::parse("sqlite://puzzles.sqlite")
                .expect("Failed to parse default database_url")),
            tactics: TacticsConfig::default(),
            srs: SrsConfig::default(),
            backup: BackupConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for TacticsConfig {
    fn default() -> Self {
        Self {
            puzzle_rating_variation_up: 0.0,
            puzzle_rating_variation_down: 0.05,
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: "./backups".into(),
            hour: NaiveTime::from_hms_opt(4, 0, 0)
                .expect("Failed to parse default backup hour"),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            initial_move_delay: 500,
            subsequent_move_delay: 250,
        }
    }
}

impl AppConfig {
    /// Load the app config from a .env file or environment variables.
    pub fn from_env() -> Result<AppConfig, Box<dyn Error>> {
        let _ = dotenvy::dotenv();

        let defaults: AppConfig = Default::default();

        Ok(Self {
            bind_interface: Self::env_var("BIND_INTERFACE")?.unwrap_or(defaults.bind_interface),
            bind_port: Self::env_var("BIND_PORT")?.unwrap_or(defaults.bind_port),
            database_url: Self::get_database_url()?.unwrap_or(defaults.database_url),
            srs: SrsConfig {
                default_ease: Self::env_var("SRS_DEFAULT_EASE")?.unwrap_or(defaults.srs.default_ease),
                minimum_ease: Self::env_var("SRS_MINIMUM_EASE")?.unwrap_or(defaults.srs.minimum_ease),
                easy_bonus: Self::env_var("SRS_EASY_BONUS")?.unwrap_or(defaults.srs.easy_bonus),
                day_end_hour: Self::env_var::<u32>("SRS_DAY_END_HOUR")?
                    .map(|day_end_hour| NaiveTime::from_hms_opt(day_end_hour, 0, 0)
                            .ok_or_else(|| format!("Invalid srs day_end_hour {}", day_end_hour)))
                    .transpose()?
                    .unwrap_or(defaults.srs.day_end_hour),
                review_order: Self::env_var::<ReviewOrder>("SRS_REVIEW_ORDER")
                    .map_err(|e| format!("{e}, possible values: {}", ReviewOrder::possible_values()))?
                    .unwrap_or(defaults.srs.review_order),
            },
            tactics: TacticsConfig {
                puzzle_rating_variation_up: Self::env_var("TACTICS_PUZZLE_RATING_VARIATION_UP")?
                    .unwrap_or(defaults.tactics.puzzle_rating_variation_up),
                puzzle_rating_variation_down: Self::env_var("TACTICS_PUZZLE_RATING_VARIATION_DOWN")?
                    .unwrap_or(defaults.tactics.puzzle_rating_variation_down),
            },
            backup: BackupConfig {
                enabled: Self::env_var("BACKUP_ENABLED")?.unwrap_or(defaults.backup.enabled),
                path: Self::env_var("BACKUP_PATH")?.unwrap_or(defaults.backup.path),
                hour: Self::env_var::<u32>("BACKUP_HOUR")?
                    .map(|day_end_hour| NaiveTime::from_hms_opt(day_end_hour, 0, 0)
                            .ok_or_else(|| format!("Invalid backup hour {}", day_end_hour)))
                    .transpose()?
                    .unwrap_or(defaults.backup.hour),
            },
            ui: UiConfig {
                initial_move_delay: Self::env_var("UI_INITIAL_MOVE_DELAY")?
                    .unwrap_or(defaults.ui.initial_move_delay),
                subsequent_move_delay: Self::env_var("UI_SUBSEQUENT_MOVE_DELAY")?
                    .unwrap_or(defaults.ui.subsequent_move_delay),
            },
        })
    }

    /// Get the database address from environment variables.
    pub fn get_database_url() -> Result<Option<DatabaseUrl>, Box<dyn Error>> {
        let db_url = Self::env_var("DATABASE_URL")?
            // Support old DB_NAME variable.
            .or(Self::env_var("SQLITE_DB_NAME")?
                .map(|db_name: String| format!("sqlite://{db_name}")))
            // Parse to URL.
            .map(|s| Ok(DatabaseUrl(Url::parse(&s)?)) as Result<_, Box<dyn Error>>)
            .transpose()?;

        Ok(db_url)
    }

    /// Read an env var and get a value, attempting to parse it to the specified type. If the
    /// variable is not set, returns the specified default.
    fn env_var<T>(key: &str) -> Result<Option<T>, Box<dyn Error>>
    where
        T: FromStr,
        <T as FromStr>::Err: Error + 'static,
    {
        match env::var(key) {
            Ok(value) => {
                let parsed = value.parse()
                    .map_err(|err| format!("Error parsing {key}: {err}"))?;
                Ok(Some(parsed))
            },
            Err(VarError::NotPresent) => Ok(None),
            Err(err) => Err(format!("Var error {key}: {err}").into())
        }
    }

    /// Get the last backup time.
    pub fn last_backup_datetime(&self) -> DateTime<Local> {
        crate::util::next_time_after(Local::now(), self.backup.hour) - Duration::days(1)
    }
}

/// The application state.
#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub user_service: UserService,
    pub tactics_service: TacticsService,
}

impl AppState {
    pub fn new(app_config: AppConfig, db: PuzzleDatabase) -> AppState {
        Self {
            user_service: UserService::new(app_config.clone(), db.clone()),
            tactics_service: TacticsService::new(app_config.clone(), db.clone()),
            app_config,
        }
    }
}
