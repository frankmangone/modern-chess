use std::{env, fmt, net::SocketAddr};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub api: ApiConfig,
    pub db: DatabaseConfig,
    pub auth: AuthConfig,
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
}

/// The private signing key. Its text is never printed: `Debug` hides it, so logging the
/// whole config cannot leak it.
#[derive(Clone)]
pub struct AuthConfig {
    pub signing_key_pem: String,
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthConfig")
            .field("signing_key_pem", &"<hidden>")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub json: bool,
    pub http_access_logs: bool,
}

type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(&|key| env::var(key).ok())
    }

    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let log_format = text(lookup, "LOG_FORMAT", "plain");
        if log_format != "plain" && log_format != "json" {
            return Err(ConfigError::InvalidValue("LOG_FORMAT", log_format));
        }

        Ok(Self {
            api: ApiConfig {
                host: text(lookup, "API_HOST", "0.0.0.0"),
                port: number(lookup, "API_PORT", 3002)?,
            },
            db: DatabaseConfig {
                url: lookup("DATABASE_URL").ok_or(ConfigError::Missing("DATABASE_URL"))?,
                max_connections: number(lookup, "DATABASE_MAX_CONNECTIONS", 8)?,
                connect_timeout_secs: number(lookup, "DATABASE_CONNECT_TIMEOUT_SECS", 5)?,
                migrate_on_startup: flag(lookup, "DB_MIGRATE_ON_STARTUP", true)?,
            },
            auth: AuthConfig {
                // Keys sit in one line of a .env file with `\n` for line breaks.
                signing_key_pem: lookup("SOCIAL_SIGNING_KEY_PEM")
                    .ok_or(ConfigError::Missing("SOCIAL_SIGNING_KEY_PEM"))?
                    .replace("\\n", "\n"),
            },
            logging: LoggingConfig {
                level: text(lookup, "RUST_LOG", "modern_chess_social_api=info,info"),
                json: log_format == "json",
                http_access_logs: flag(lookup, "HTTP_ACCESS_LOGS", true)?,
            },
        })
    }

    pub fn api_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.api.host, self.api.port)
            .parse()
            .map_err(|_| ConfigError::InvalidSocketAddr("API_HOST/API_PORT"))
    }
}

fn text(lookup: Lookup, key: &str, default: &str) -> String {
    lookup(key).unwrap_or_else(|| default.to_string())
}

fn number<T: std::str::FromStr>(
    lookup: Lookup,
    key: &'static str,
    default: T,
) -> Result<T, ConfigError> {
    match lookup(key) {
        Some(raw) => raw.parse().map_err(|_| ConfigError::InvalidValue(key, raw)),
        None => Ok(default),
    }
}

fn flag(lookup: Lookup, key: &'static str, default: bool) -> Result<bool, ConfigError> {
    match lookup(key).as_deref() {
        None => Ok(default),
        Some("true" | "1" | "yes") => Ok(true),
        Some("false" | "0" | "no") => Ok(false),
        Some(other) => Err(ConfigError::InvalidValue(key, other.to_string())),
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
