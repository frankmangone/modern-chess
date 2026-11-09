use super::env::{flag, required, text};
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
        if !is_plain_http(&self.public_url) {
            return Err(ConfigError::InvalidValue(
                "IMAGES_PUBLIC_URL",
                "must be an http:// or https:// address without a query or fragment".to_string(),
            ));
        }
        Ok(())
    }
}

/// How the cover tool reaches the image store (`IMAGES_*`). Kept apart from [`ImagesConfig`]:
/// only the tool that uploads holds credentials that can write, never the web server.
#[derive(Clone)]
pub struct ImageStorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    /// Buckets addressed by path, as MinIO does; on for the local store.
    pub force_path_style: bool,
}

impl ImageStorageConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let config = Self {
            endpoint: text(lookup, "IMAGES_ENDPOINT", "http://127.0.0.1:9000")
                .trim_end_matches('/')
                .to_string(),
            region: text(lookup, "IMAGES_REGION", "us-east-1"),
            bucket: text(lookup, "IMAGES_BUCKET", "modern-chess"),
            access_key: required(lookup, "IMAGES_ACCESS_KEY")?,
            secret_key: required(lookup, "IMAGES_SECRET_KEY")?,
            force_path_style: flag(lookup, "IMAGES_FORCE_PATH_STYLE", true)?,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        let invalid =
            |key: &'static str, why: &str| Err(ConfigError::InvalidValue(key, why.to_string()));
        if !is_plain_http(&self.endpoint) {
            return invalid(
                "IMAGES_ENDPOINT",
                "must be an http:// or https:// address without a query or fragment",
            );
        }
        if !valid_bucket(&self.bucket) {
            return invalid(
                "IMAGES_BUCKET",
                "must be 3 to 63 lowercase letters, digits, dots or hyphens",
            );
        }
        if self.region.trim().is_empty() {
            return invalid("IMAGES_REGION", "must not be blank");
        }
        // The values themselves are never repeated: one of them is a secret.
        if self.access_key.trim().is_empty() {
            return invalid("IMAGES_ACCESS_KEY", "must not be blank");
        }
        if self.secret_key.trim().is_empty() {
            return invalid("IMAGES_SECRET_KEY", "must not be blank");
        }
        Ok(())
    }
}

// A stray `{:?}` must not put the secret in a log.
impl std::fmt::Debug for ImageStorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageStorageConfig")
            .field("endpoint", &self.endpoint)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .field("access_key", &self.access_key)
            .field("secret_key", &"<hidden>")
            .field("force_path_style", &self.force_path_style)
            .finish()
    }
}

fn is_plain_http(address: &str) -> bool {
    let host = address
        .strip_prefix("http://")
        .or_else(|| address.strip_prefix("https://"))
        .unwrap_or("");
    !host.is_empty()
        && !host.starts_with('/')
        && !address.contains(['?', '#'])
        && !address.contains(char::is_whitespace)
}

fn valid_bucket(name: &str) -> bool {
    (3..=63).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && !name.starts_with(['-', '.'])
        && !name.ends_with(['-', '.'])
}
