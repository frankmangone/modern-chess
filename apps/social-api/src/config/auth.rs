use std::fmt;

use super::env::required;
use super::{ConfigError, Lookup};

/// The private signing key. Its text is never printed: `Debug` hides it, so logging the
/// whole config cannot leak it. It keeps the name `SOCIAL_SIGNING_KEY_PEM` on purpose: no
/// other app asks for it, and the game process is tested never to read it.
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

impl AuthConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        // Keys sit in one line of a .env file with `\n` for line breaks.
        let signing_key_pem = required(lookup, "SOCIAL_SIGNING_KEY_PEM")?.replace("\\n", "\n");
        let config = Self { signing_key_pem };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Never repeats the value: it is a secret.
        if self.signing_key_pem.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "SOCIAL_SIGNING_KEY_PEM",
                "must not be blank".to_string(),
            ));
        }
        Ok(())
    }
}
