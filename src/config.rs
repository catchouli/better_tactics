use std::env::{self, VarError};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

/// The application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_interface: IpAddr,
    pub bind_port: u16,
    pub db_name: String,
    pub srs: SrsConfig,
}

#[derive(Debug, Clone)]
pub struct SrsConfig {
    pub default_ease: f64,
    pub minimum_ease: f64,
    pub easy_bonus: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind_interface: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            bind_port: 3030,
            db_name: "puzzles.sqlite".to_string(),
            srs: SrsConfig {
                default_ease: 2.5,
                minimum_ease: 1.3,
                easy_bonus: 1.3,
            },
        }
    }
}

impl AppConfig {
    /// Load the app config from a .env file or environment variables.
    pub fn from_env() -> Result<AppConfig, Box<dyn Error>> {
        let _ = dotenvy::dotenv();

        let defaults: AppConfig = Default::default();

        Ok(Self {
            bind_interface: Self::env_var_or_default("BIND_INTERFACE", defaults.bind_interface)?,
            bind_port: Self::env_var_or_default("BIND_PORT", defaults.bind_port)?,
            db_name: Self::env_var_or_default("SQLITE_DB_NAME", defaults.db_name)?,
            srs: SrsConfig {
                default_ease: Self::env_var_or_default("SRS_DEFAULT_EASE", defaults.srs.default_ease)?,
                minimum_ease: Self::env_var_or_default("SRS_MINIMUM_EASE", defaults.srs.minimum_ease)?,
                easy_bonus: Self::env_var_or_default("SRS_EASY_BONUS", defaults.srs.easy_bonus)?,
            },
        })
    }

    /// Read an env var and get a value, attempting to parse it to the specified type. If the
    /// variable is not set, returns the specified default.
    fn env_var_or_default<T>(key: &str, default: T) -> Result<T, Box<dyn Error>>
    where
        T: FromStr,
        <T as FromStr>::Err: Error + 'static,
    {
        match env::var(key) {
            Ok(value) => Ok(value.parse()?),
            Err(VarError::NotPresent) => Ok(default),
            Err(err) => Err(err.into())
        }
    }
}
