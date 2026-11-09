use super::env::text;
use super::{ConfigError, Lookup};

/// Where game images are loaded from (`IMAGES_*`). The API only builds addresses from it: it
/// holds no storage credentials, which belong to the tool that uploads covers.
#[derive(Debug, Clone)]
pub struct ImagesConfig {
    /// The address that stored image keys sit under, such as `https://img.example.com/covers`.
    pub public_url: String,
}

impl ImagesConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        // Without a setting, point at the MinIO that `make storage-up` starts.
        let raw = text(
            lookup,
            "IMAGES_PUBLIC_URL",
            "http://127.0.0.1:9000/modern-chess",
        );
        let config = Self {
            public_url: raw.trim_end_matches('/').to_string(),
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let host = self
            .public_url
            .strip_prefix("http://")
            .or_else(|| self.public_url.strip_prefix("https://"))
            .unwrap_or("");
        let plain = !host.is_empty()
            && !host.starts_with('/')
            && !self.public_url.contains(['?', '#'])
            && !self.public_url.contains(char::is_whitespace);
        if !plain {
            return Err(ConfigError::InvalidValue(
                "IMAGES_PUBLIC_URL",
                "must be an http:// or https:// address without a query or fragment".to_string(),
            ));
        }
        Ok(())
    }
}
