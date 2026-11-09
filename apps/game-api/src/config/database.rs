use super::env::{flag, parsed, positive, required};
use super::{ConfigError, Lookup};

/// The game database and what happens to it at start (`DATABASE_*`).
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub migrate_on_startup: bool,
    pub seed_defaults_on_startup: bool,
}

impl DatabaseConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let config = Self {
            url: required(lookup, "DATABASE_URL")?,
            max_connections: parsed(lookup, "DATABASE_MAX_CONNECTIONS", 8_u32)?,
            connect_timeout_secs: parsed(lookup, "DATABASE_CONNECT_TIMEOUT_SECS", 5_u64)?,
            migrate_on_startup: flag(lookup, "DATABASE_MIGRATE_ON_STARTUP", true)?,
            seed_defaults_on_startup: flag(lookup, "DATABASE_SEED_DEFAULT_SPECS", true)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // The url holds a password, so the message never repeats it.
        if !(self.url.starts_with("postgres://") || self.url.starts_with("postgresql://")) {
            return Err(ConfigError::InvalidValue(
                "DATABASE_URL",
                "must start with postgres:// or postgresql://".to_string(),
            ));
        }
        positive("DATABASE_MAX_CONNECTIONS", u64::from(self.max_connections))?;
        positive("DATABASE_CONNECT_TIMEOUT_SECS", self.connect_timeout_secs)?;
        Ok(())
    }
}
