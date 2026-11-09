use super::env::required;
use super::{ConfigError, Lookup};

/// How this app recognises callers (`AUTH_*`). It holds the public half of the signing key
/// only: the private half (`SOCIAL_SIGNING_KEY_PEM`) belongs to social-api and is never read
/// here.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub public_key_pem: String,
}

impl AuthConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        // Keys sit in one line of a .env file with `\n` for line breaks.
        let public_key_pem = required(lookup, "AUTH_PUBLIC_KEY_PEM")?.replace("\\n", "\n");
        let config = Self { public_key_pem };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.public_key_pem.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "AUTH_PUBLIC_KEY_PEM",
                "must not be blank".to_string(),
            ));
        }
        Ok(())
    }
}
