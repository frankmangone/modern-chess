//! The app's settings, one file per domain. A domain reads only variables that start with its
//! own prefix (`API_`, `DATABASE_`, `LOGGING_`) and checks its own values, so a bad setting
//! stops the app at start with a message naming the variable. The signing key keeps its
//! unmistakable name, `SOCIAL_SIGNING_KEY_PEM`, so no other app can pick it up by accident.

mod api;
mod auth;
mod database;
mod env;
mod logging;

use std::env as process_env;

use thiserror::Error;

pub use api::ApiConfig;
pub use auth::AuthConfig;
pub use database::DatabaseConfig;
pub use env::Lookup;
pub use logging::{LogFormat, LoggingConfig};

#[derive(Debug, Clone)]
pub struct Config {
    pub api: ApiConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(&|key| process_env::var(key).ok())
    }

    /// Reads every setting through `lookup`, so a test can see exactly which variables the
    /// app asks for.
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        Ok(Self {
            api: ApiConfig::from_lookup(lookup)?,
            database: DatabaseConfig::from_lookup(lookup)?,
            auth: AuthConfig::from_lookup(lookup)?,
            logging: LoggingConfig::from_lookup(lookup)?,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    InvalidValue(&'static str, String),
}
