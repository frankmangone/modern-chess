use std::{env, net::SocketAddr};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub api: ApiConfig,
    pub db: DatabaseConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub migrate_on_startup: bool,
    pub seed_defaults_on_startup: bool,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub http_access_logs: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Plain,
    Json,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_host = env_or_default("API_HOST", "0.0.0.0");
        let api_port = parse_u16_env("API_PORT", 3001)?;

        let db_url = env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        let db_max_connections = parse_u32_env("DATABASE_MAX_CONNECTIONS", 8)?;
        let db_connect_timeout_secs = parse_u64_env("DATABASE_CONNECT_TIMEOUT_SECS", 5)?;
        let migrate_on_startup = parse_bool_env("DB_MIGRATE_ON_STARTUP", true)?;
        let seed_defaults_on_startup = parse_bool_env("DB_SEED_DEFAULT_SPECS", true)?;

        let log_level = env_or_default("RUST_LOG", "modern_chess_game_api=info,info");
        let log_format = match env_or_default("LOG_FORMAT", "plain").as_str() {
            "plain" => LogFormat::Plain,
            "json" => LogFormat::Json,
            other => return Err(ConfigError::InvalidValue("LOG_FORMAT", other.to_string())),
        };
        let http_access_logs = parse_bool_env("HTTP_ACCESS_LOGS", true)?;

        Ok(Self {
            api: ApiConfig {
                host: api_host,
                port: api_port,
            },
            db: DatabaseConfig {
                url: db_url,
                max_connections: db_max_connections,
                connect_timeout_secs: db_connect_timeout_secs,
                migrate_on_startup,
                seed_defaults_on_startup,
            },
            logging: LoggingConfig {
                level: log_level,
                format: log_format,
                http_access_logs,
            },
        })
    }

    pub fn api_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.api.host, self.api.port)
            .parse()
            .map_err(|_| ConfigError::InvalidSocketAddr("API_HOST/API_PORT"))
    }
}

fn env_or_default(key: &'static str, default: &'static str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse_bool_env(key: &'static str, default: bool) -> Result<bool, ConfigError> {
    match env::var(key) {
        Ok(v) => match v.as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(ConfigError::InvalidValue(key, v)),
        },
        Err(_) => Ok(default),
    }
}

fn parse_u16_env(key: &'static str, default: u16) -> Result<u16, ConfigError> {
    match env::var(key) {
        Ok(v) => v
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidValue(key, v)),
        Err(_) => Ok(default),
    }
}

fn parse_u32_env(key: &'static str, default: u32) -> Result<u32, ConfigError> {
    match env::var(key) {
        Ok(v) => v
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidValue(key, v)),
        Err(_) => Ok(default),
    }
}

fn parse_u64_env(key: &'static str, default: u64) -> Result<u64, ConfigError> {
    match env::var(key) {
        Ok(v) => v
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue(key, v)),
        Err(_) => Ok(default),
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    InvalidValue(&'static str, String),
    #[error("invalid socket address from {0}")]
    InvalidSocketAddr(&'static str),
}
