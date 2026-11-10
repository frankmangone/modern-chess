use std::fmt;

use modern_chess_social::provider::ProviderConfig;

use super::env::required;
use super::{ConfigError, Lookup};

/// The private signing key. Its text is never printed: `Debug` hides it, so logging the
/// whole config cannot leak it. It keeps the name `SOCIAL_SIGNING_KEY_PEM` on purpose: no
/// other app asks for it, and the game process is tested never to read it.
#[derive(Clone)]
pub struct AuthConfig {
    pub signing_key_pem: String,
    /// The outside sign-in provider, when one is set up. Without it only guests can sign in.
    /// Its settings are public (an address and a name), so unlike the key they may be printed.
    pub provider: Option<ProviderConfig>,
}

impl fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthConfig")
            .field("signing_key_pem", &"<hidden>")
            .field("provider", &self.provider)
            .finish()
    }
}

impl AuthConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        // Keys sit in one line of a .env file with `\n` for line breaks.
        let signing_key_pem = required(lookup, "SOCIAL_SIGNING_KEY_PEM")?.replace("\\n", "\n");
        let provider = provider_from_lookup(lookup)?;
        let config = Self {
            signing_key_pem,
            provider,
        };
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

const NAME_MAX_LEN: usize = 32;

/// `SOCIAL_PROVIDER_ISSUER` turns sign-in on; the rest describe it. Nothing here is a secret:
/// verifying a token takes only the provider's public keys.
fn provider_from_lookup(lookup: Lookup) -> Result<Option<ProviderConfig>, ConfigError> {
    let Some(issuer) = lookup("SOCIAL_PROVIDER_ISSUER") else {
        return Ok(None);
    };
    // Kept exactly as given: tokens carry the issuer text itself, and some providers (Auth0) end
    // theirs with a slash while others (Clerk) do not.
    let issuer = issuer.trim().to_string();
    secure_address("SOCIAL_PROVIDER_ISSUER", &issuer)?;

    let name = required(lookup, "SOCIAL_PROVIDER_NAME")?;
    let name_ok = !name.is_empty()
        && name.len() <= NAME_MAX_LEN
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !name_ok {
        return Err(ConfigError::InvalidValue(
            "SOCIAL_PROVIDER_NAME",
            "must be 1 to 32 characters: lowercase letters, digits, - and _".to_string(),
        ));
    }

    let jwks_url = match lookup("SOCIAL_PROVIDER_JWKS_URL") {
        Some(url) => url.trim().to_string(),
        None => format!("{}/.well-known/jwks.json", issuer.trim_end_matches('/')),
    };
    secure_address("SOCIAL_PROVIDER_JWKS_URL", &jwks_url)?;

    let authorized_parties = lookup("SOCIAL_PROVIDER_AUTHORIZED_PARTIES")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect();

    Ok(Some(ProviderConfig {
        name,
        issuer,
        jwks_url,
        authorized_parties,
    }))
}

/// An https address, or http to this machine for local development. The host is compared
/// whole, so `http://localhost.evil.test` is not local.
fn secure_address(key: &'static str, address: &str) -> Result<(), ConfigError> {
    let local = ["http://localhost", "http://127.0.0.1"]
        .iter()
        .any(|prefix| {
            address.strip_prefix(prefix).is_some_and(|rest| {
                rest.is_empty() || rest.starts_with(':') || rest.starts_with('/')
            })
        });
    if address.starts_with("https://") || local {
        return Ok(());
    }
    Err(ConfigError::InvalidValue(
        key,
        "must be an https address".to_string(),
    ))
}
