use super::env::{flag, text};
use super::{ConfigError, Lookup};

/// What the app writes to its log (`LOGGING_*`; the level keeps the library's own `RUST_LOG`).
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

impl LoggingConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let format = match text(lookup, "LOGGING_FORMAT", "plain").as_str() {
            "plain" => LogFormat::Plain,
            "json" => LogFormat::Json,
            other => {
                return Err(ConfigError::InvalidValue(
                    "LOGGING_FORMAT",
                    format!("{other} (expected plain or json)"),
                ))
            }
        };
        Ok(Self {
            level: text(lookup, "RUST_LOG", "modern_chess_social_api=info,info"),
            format,
            http_access_logs: flag(lookup, "LOGGING_HTTP_ACCESS_LOGS", true)?,
        })
    }
}
