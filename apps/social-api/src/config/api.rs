use std::net::SocketAddr;

use super::env::{parsed, positive, text};
use super::{ConfigError, Lookup};

/// Where the HTTP server listens (`API_*`).
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

impl ApiConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let config = Self {
            host: text(lookup, "API_HOST", "0.0.0.0"),
            port: parsed(lookup, "API_PORT", 3002_u16)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        positive("API_PORT", u64::from(self.port))?;
        format!("{}:{}", self.host, self.port)
            .parse::<SocketAddr>()
            .map_err(|_| {
                ConfigError::InvalidValue("API_HOST", "must be an IP address".to_string())
            })?;
        Ok(())
    }

    /// The address to listen on. `validate` has already proved it parses.
    #[allow(
        clippy::expect_used,
        reason = "validate() runs when the config is read, before this is called at startup"
    )]
    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("checked when the config was read")
    }
}
