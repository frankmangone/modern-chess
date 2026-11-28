use super::{ConfigError, Lookup};

/// Where the game process is (`SOCIAL_GAME_API_URL`), so discussion can ask which game pages are
/// open. Leave it out and discussion is off: its routes answer 503.
#[derive(Debug, Clone, Default)]
pub struct GameApiConfig {
    /// The game process's address up to and including `/api/game`, for example
    /// `http://localhost:3001/api/game`.
    pub url: Option<String>,
}

impl GameApiConfig {
    pub fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let Some(url) = lookup("SOCIAL_GAME_API_URL")
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())
        else {
            return Ok(Self { url: None });
        };
        // Inside the deployment this is often plain http on a private network, so unlike the
        // sign-in provider it is not required to be https.
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(ConfigError::InvalidValue(
                "SOCIAL_GAME_API_URL",
                "must start with http:// or https://".to_string(),
            ));
        }
        Ok(Self { url: Some(url) })
    }
}
